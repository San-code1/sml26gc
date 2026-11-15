# Arquitectura

Un **workspace de Cargo** con piezas independientes (emulador, conocimiento del cartucho,
juego, editor, herramientas de investigación), con arquitectura hexagonal (puertos y
adaptadores). Salió de `legacy/gb_port` (un crate único con todo mezclado) sin cambiar el
comportamiento: `regression/golden.txt` lo vigila.

---

## 1. Estructura del workspace

```
New Mario Bros/
├─ Cargo.toml                 workspace (versiones de dependencias compartidas)
├─ crates/
│  ├─ gb-core/                DOMINIO · la Game Boy emulada. Sin dependencias, sin E/S
│  ├─ cart-model/             DOMINIO · lo que sabemos del cartucho: niveles, objetos,
│  │                          mapa del mundo, parches de ROM, mapa de RAM, casos de uso
│  │                          sobre la consola (arrancar al mapa, entrar en nivel...)
│  └─ host/                   ADAPTADORES compartidos: localizar la ROM, rutas de
│                             partidas/ediciones/ajustes, ficheros (std::fs)
├─ game/                      APLICACIÓN · el juego  → game.exe
├─ editor/                    APLICACIÓN · el editor → editor.exe
├─ tools/research/            Herramienta CLI de ingeniería inversa (disasm, watch, trace...)
├─ regression/                Huella dorada + test que la verifica contra el código nuevo
└─ legacy/gb_port/            Referencia intacta; se borra al terminar
```

Reglas de dependencia (las flechas sólo van hacia dentro):

```
 game ─────┐                     tools/research ─┐
 editor ───┼──► host ──► cart-model ──► gb-core ◄─┘
           └──────────────►┘
```

- `gb-core` no conoce el juego, ni ficheros, ni macroquad, ni cpal.
- `cart-model` no conoce ficheros ni interfaz: trabaja con `&[u8]` y con `GameBoy`.
- `host` no conoce macroquad ni cpal.
- `game` y `editor` no dependen el uno del otro. Se comunican sólo por un **contrato**
  (ver §6).

---

## 2. `gb-core`: la consola

Hoy: `cpu`, `bus`, `ppu`, `apu`, `cart`, con todos los campos públicos
(`gb.bus.cart.ram`, `gb.bus.ppu.fb`...) y el código de investigación metido dentro del bus.

API pública:

```rust
pub struct GameBoy<O: BusObserver = NoObserver> { /* campos privados */ }
pub type Gb = GameBoy<NoObserver>;

Gb::new(rom, sample_rate) / GameBoy::with_observer(rom, sample_rate, observer)
run_frame() · run_frame_with(Buttons) · set_buttons(Buttons)
read(addr) · write(addr, v)                  // a través del bus, como el programa
screen() -> &Screen                           // 160x144 tonos 0-3
audio() -> &[f32] · clear_audio() · sample_rate()
video() -> &Ppu        // vram(), oam(), lcdc(), bgp(), obp(n), scx()
wram() · cart() / cart_mut()  // Cart: rom(), ram(), ram_mut(), take_ram(), set_ram(),
                              //       has_battery(), needs_saving(), mark_saved(), title()...
pc() · observer() / observer_mut()
```

`Buttons` es un conjunto tipado (`Buttons::START | Buttons::A`). CPU, bus, APU y sus campos ya
no son visibles fuera del crate.

- **`BusObserver`** (genérico, despacho estático; `NoObserver` tiene `ENABLED = false` y el
  compilador elimina las llamadas): `on_read(pc, bank, addr)`, `on_write(pc, bank, addr, v)`.
  Para ganchos de mods se podría añadir `on_execute(pc)`. Sustituye a `watch`, `watch_log`,
  `read_watch`... que había en el bus (ahora es `Probe`, en `tools/research`) y es **la base del
  sistema de ganchos** para futuros mods.
- **Rendimiento**: nada de `dyn Trait` en el bucle de la CPU ni en los accesos a memoria.
  El MBC sigue siendo un `enum` (conjunto cerrado de hardware real): aquí SOLID se aplica
  con criterio, no como dogma.
- **Posible punto de extensión gráfico**: la pantalla podría incluir, además del tono, de qué capa
  viene cada píxel (fondo, ventana, sprite con OBP0/OBP1) y qué tile lo pintó. Es un búfer
  paralelo: no cambia el búfer actual, así que la huella dorada no se ve afectada.
  Permitiría paletas por capa y sustituir gráficos.

## 3. `cart-model`: el conocimiento del cartucho

Hoy: `levels`, `objects`, `world`, y parte de `machine.rs`
(`boot_to_map`, `enter_level`, `back_to_map`...), con direcciones mágicas repartidas.

Propuesta (módulos):

| Módulo | Contenido |
|---|---|
| `rom` | `offset(bank, addr)`, `word`, `set_word`. Las funciones reciben `&[u8]`: un newtype `RomImage` no aportaba nada y obligaba a tocar todo el editor |
| `ram_map` | Direcciones con nombre: `GAME_MODE = 0xFF9B`, `LEVEL_NUMBER = 0xA269`, `MAP_POINT = 0xA840`... y `enum GameMode { FileSelect, Map, Level, LosingLife, GameOver, ... }` |
| `levels` | `Header`, `map_location`, `decode`/`encode` (funciones puras), `sharing` |
| `objects` | `LevelObject`, codec de 3 bytes, lectura/escritura de listas |
| `world` | Imagen del mapa 64x64, tabla punto→nivel |
| `patch` | Ampliación a 1 MB, parche de objetos, `can_expand`, `upgrade` |
| `layout` | Capacidad por banco, empaquetado de mapas y objetos (`store_bank`, `space`) |
| `extra_levels` | Niveles nuevos (32 en adelante): guardados en bancos libres de la copia de 1 MB y jugados en lugar de un nivel base (`writes_for`, `apply`) |
| `original_points` | Dónde dibuja el juego sus puntos y sus caminos (medidos), para el editor |
| `art` | `LevelArt`, `WorldArt`, captura de sprites (`Look`) a partir de una `GameBoy` |
| `live` | Lo que el juego en marcha tiene cargado: mapa del nivel, objetos, posición del jugador |
| `error` | `CartError` (`UnknownVersion`, `ZoneFull`, `ObjectsFull`, `TooManyLevels`, `LevelTooBig`, `LevelsFull`), `BankUsage`, `ObjectsUsage` |
| `driver` | `GameDriver`: casos de uso sobre la consola. `boot_to_map`, `enter_level`, `enter_level_anywhere`, `back_to_map`, `back_to_file_select`, `mode()` |

- Errores tipados (`enum CartError`) en lugar de `String`; los textos son los de siempre.
- Sin `std::fs`: los ficheros están en `host`.
- Con tests unitarios que no necesitan la ROM.

## 4. `game/`: el juego (hexagonal)

```
game/src/
├─ domain/            Lógica pura, testeable sin ventana ni ficheros
│  ├─ keys.rs         Key (propio, no macroquad), teclas asignables, nombres en settings.cfg,
│  │                  etiquetas, Buttons desde las asignaciones
│  ├─ settings.rs     Settings y su texto key=value (from_text / to_text)
│  ├─ i18n.rs         Textos EN/ES/JA
│  ├─ overlay.rs      Fuente bitmap, Canvas y la etiqueta "ESC: MENU"
│  └─ menu.rs         Menú de pausa → Action; edita Settings, no guarda ni toca la ventana
├─ application/
│  ├─ preferences.rs  Settings + guardar + seguir la pantalla completa
│  └─ session.rs      GameSession::frame(dt, &dyn Keyboard, &dyn AudioOut) → Flow; niveles
│                     nuevos (32+) puestos en lugar de su nivel base cuando el juego los pide
├─ ports.rs           Keyboard, AudioOut, Window, SettingsStore, SaveStore
├─ adapters/
│  ├─ macroquad_keyboard.rs  Key ↔ KeyCode
│  ├─ macroquad_display.rs   Paleta → textura, escalado, error en pantalla, Window
│  ├─ cpal_audio.rs          AudioOut con cpal
│  ├─ fs_stores.rs           settings.cfg, saves/*.sav (vía host) y DefaultSettings
│  ├─ macroquad_picker.rs    Pantalla de inicio: elegir el cartucho (se recuerda)
│  └─ win32.rs               Ventana al fondo en modo comprobación; diálogo Abrir
├─ checks.rs, selftest.rs    Comprobaciones sin ventana (--research, --self-test, --dump)
└─ main.rs            Raíz de composición: argumentos, adaptadores, bucle de la ventana
```

Principios aplicados:
- **S**: el menú no sabe guardar ajustes; `Preferences` guarda y aplica la pantalla completa;
  la sesión coordina; cada adaptador traduce una sola tecnología.
- **O**: nuevas paletas, filtros o capas de dibujo se añaden como nuevas implementaciones,
  sin tocar la sesión.
- **L/I**: puertos pequeños, sustituidos en los tests por dobles (`FakeKeys`, `Silence`,
  `Record`).
- **D**: `GameSession` depende de traits; `main.rs` decide las implementaciones (en modo
  `--screenshot`: audio mudo y `DefaultSettings`, que nunca escribe el fichero del jugador).
- `domain/` y `application/` no usan macroquad, cpal, `host` ni `std::fs`.
- Tests: teclas, ajustes, seguimiento de niveles, menú con teclado falso y una sesión
  completa (encender → nivel → menú → pantalla completa → guardar) con puertos falsos.

## 5. `editor/`: el editor (hexagonal)

Antes: `editor.rs` de 1.726 líneas con un único `struct Editor` de ~45 campos que hacía
interfaz, lógica de edición, deshacer, guardado, captura de gráficos y lanzamiento del juego.

```
editor/src/
├─ domain/            Sin ventana ni ficheros
│  ├─ document.rs     EditorDocument: original + copia en construcción, 32 niveles y los
│  │                  nuevos (bloques + objetos + historial), mapa del mundo, puntos,
│  │                  marcas de "sin guardar", espacio libre y commit() a la imagen del cartucho
│  ├─ history.rs      History<T>: deshacer/rehacer genérico (niveles y mapa del mundo)
│  └─ tools.rs        trait Tool + GridCanvas → Pencil, BoxFill
│                     (nueva herramienta = nueva impl; sirve en niveles y en el mapa)
├─ application/
│  ├─ save.rs         commit + escribir la copia (vía puerto); errores tipados
│  └─ art_service.rs  Gráficos del juego: consola arrancada al mapa, niveles, tiles, objetos
├─ ports.rs           EditedCopyStore, GameLauncher
├─ adapters/
│  ├─ fs_repository.rs     Abrir original + copia (host::paths), guardar, idioma
│  └─ process_launcher.rs  game.exe <copia> [--level N]
├─ ui/                macroquad, por paneles (comparten el struct Editor de ui/mod.rs)
│  ├─ mod.rs          Estado de pantalla, navegación (zoom/cámara), guardar/jugar, textos de estado
│  ├─ level_view.rs   Vista del nivel, vista general, panel lateral
│  ├─ objects_view.rs Objetos: poner, arrastrar, cambiar tipo, borrar
│  ├─ palette.rs      Paletas de bloques y de tipos de objeto
│  ├─ world_view.rs   Mapa del mundo: dibujo y puntos originales
│  ├─ custom_view.rs  "Tu mapa"
│  ├─ status_bar.rs   Mensaje y espacio de zona/objetos
│  └─ widgets.rs      Colores, botón, wrap, texturas a partir de los gráficos capturados
└─ main.rs            Raíz de composición
```

Tests: historial, herramientas con un lienzo falso, y documento con la ROM real (pintar,
deshacer/rehacer, objeto, tile, commit y releer como lo hará el juego; restaurar original).

## 5b. `tools/research/`

```
tools/research/
├─ src/main.rs        Tabla COMMANDS (nombre → función que lee sus argumentos) y Args
├─ src/console.rs     Cargar la ROM, un frame de guion, instantáneas de RAM, pokes, shown()
├─ src/probe.rs       Probe: BusObserver que registra escrituras y lecturas de ROM
├─ src/disasm.rs      Desensamblador SM83 (con tests) y tablas de la ROM
├─ src/memory.rs      modes, diff, poke, leave, cmpmap, reads, find, watch, trace
├─ src/checks.rs      levels, storecheck, objcheck, worldcheck
├─ src/rom_views.rs   free, objects, worldshot, looks
└─ all_commands.sh    Ejecuta los 20 comandos y guarda su salida, para comparar versiones
```

## 6. Contrato entre juego y editor

Lo único que comparten fuera de los crates comunes (se mantiene **compatible** con lo actual,
para que partidas y ediciones existentes sigan sirviendo):

| Elemento | Formato |
|---|---|
| Lanzar el juego | `game.exe <rom> --level <N>` (sin rom: pantalla para elegirlo) |
| Copia editada | `edits/<nombre> (edit).gb` |
| Partidas | `saves/<nombre>.sav` |
| Último cartucho elegido | `last_cartridge.txt` |
| Ajustes (idioma lo lee también el editor) | `settings.cfg` |

Las rutas y nombres viven en `host::paths` (`save_path`, `edited_copy_path`,
`last_cartridge_path`, `settings_path`, `game_exe`), así ambos programas los calculan igual. El
resto de `host`: `rom_locator` (`find_rom`, `require_rom` con un mensaje claro si falta el
cartucho, `last_rom`, `remember_rom`), `battery`, `settings_file` y `png_out`.

## 7. Estrategia de pruebas

1. **Huella dorada** (`regression/`): debe dar `golden.txt` idéntico; un cambio de
   comportamiento buscado se refleja actualizando el fichero a propósito.
2. **Tests unitarios** del dominio puro: codecs RLE y de objetos, `History`, herramientas,
   máquina de estados del menú, niveles nuevos desde el mapa original.
3. **Rendimiento**: frames por segundo del emulador nuevo vs. legacy (no debe empeorar).
4. **Manual / capturas**: menú y editor, comparando capturas con las del legacy.
