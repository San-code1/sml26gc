# Arquitectura propuesta (borrador para revisión)

Objetivo: convertir `legacy/gb_port` (un crate único con emulador, juego, editor y herramientas
de investigación mezclados) en un **workspace de Cargo** con piezas independientes, aplicando
SOLID y arquitectura hexagonal (puertos y adaptadores), **sin cambiar el comportamiento**:
`regression/golden.txt` debe seguir saliendo idéntico en cada fase.

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

Hecho (fase 1). API pública:

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
  Para los ganchos de mods se añadirá `on_execute(pc)` (fase 7). Sustituye a `watch`, `watch_log`,
  `read_watch`... que había en el bus (ahora es `Probe`, en `tools/research`) y es **la base del
  sistema de ganchos** para futuros mods.
- **Rendimiento**: nada de `dyn Trait` en el bucle de la CPU ni en los accesos a memoria.
  El MBC sigue siendo un `enum` (conjunto cerrado de hardware real): aquí SOLID se aplica
  con criterio, no como dogma.
- **Punto de extensión gráfico** (fase 7): la pantalla podrá incluir, además del tono, de qué capa
  viene cada píxel (fondo, ventana, sprite con OBP0/OBP1) y qué tile lo pintó. Es un búfer
  paralelo: no cambia el búfer actual, así que la huella dorada no se ve afectada.
  Permite paletas por capa y sustituir gráficos (fase 7).

## 3. `cart-model`: el conocimiento del cartucho

Hoy: `levels`, `objects`, `world`, `custom_map`, y parte de `machine.rs`
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
| `custom_map` | Formato de texto del mapa propio (parse/serialize, vecinos, caminos) |
| `art` | `LevelArt`, `WorldArt`, captura de sprites (`Look`) a partir de una `GameBoy` |
| `live` | Lo que el juego en marcha tiene cargado: mapa del nivel, objetos, posición del jugador |
| `error` | `CartError` (`UnknownVersion`, `ZoneFull`, `ObjectsFull`), `BankUsage`, `ObjectsUsage` |
| `driver` | `GameDriver`: casos de uso sobre la consola. `boot_to_map`, `enter_level`, `enter_level_anywhere`, `back_to_map`, `back_to_file_select`, `mode()` |

- Errores tipados (`enum CartError`) en lugar de `String`; los textos son los de siempre.
- Sin `std::fs`: leer/guardar `.points.txt` está en `host` (`load_custom_map`, `save_custom_map`).
- Hecho (fase 2), con 14 tests unitarios que no necesitan la ROM.

## 4. `game/`: el juego (hexagonal)

Hoy: `main.rs` mezcla bucle, entrada, audio, guardado, progreso del mapa propio, menú y dibujo;
`menu.rs` mezcla estado del menú, textos, fuente y ajustes con su fichero.

```
game/src/
├─ domain/            Lógica pura, testeable sin ventana
│  ├─ settings.rs     Settings (teclas, paleta, volumen, idioma, pantalla)
│  ├─ menu.rs         Máquina de estados del menú de pausa → Action
│  ├─ i18n.rs         Textos EN/ES/JA
│  ├─ overlay.rs      Fuente bitmap y dibujo sobre el Frame (menú, aviso)
│  ├─ map_view.rs     Navegación por el mapa propio
│  └─ progress.rs     Seguimiento de nivel superado (vida perdida / volver al mapa)
├─ application/
│  └─ session.rs      GameSession: une consola + menú + mapa propio + progreso.
│                     Recibe puertos, no conoce macroquad ni cpal.
├─ ports.rs           trait Display, AudioOut, InputSource, SettingsStore,
│                     SaveStore, ProgressStore, Clock
├─ adapters/
│  ├─ macroquad_display.rs   Paleta → RGBA → textura, escalado, pantalla completa
│  ├─ macroquad_input.rs     Teclado → Buttons + teclas de menú
│  ├─ cpal_audio.rs          Cola de audio
│  ├─ fs_stores.rs           (re-exporta host) settings.cfg, saves/, progress.txt
│  └─ win32.rs               Ventana al fondo en modo check
└─ main.rs            Raíz de composición: crea adaptadores, los inyecta, arranca
```

Principios aplicados:
- **S**: cada fichero tiene una única razón para cambiar (el menú no sabe guardar ajustes).
- **O**: nuevas paletas, filtros o capas de dibujo se añaden como nuevas implementaciones,
  sin tocar la sesión.
- **L/I**: puertos pequeños (`AudioOut` sólo `push` y `queued`), fáciles de sustituir por
  dobles de prueba (`SilentAudio`, `ScriptedInput`).
- **D**: `GameSession` depende de traits; `main.rs` decide las implementaciones.
- `--self-test`, `--screenshot`, `--dump` pasan a usar adaptadores falsos o sin ventana.

## 5. `editor/`: el editor (hexagonal)

Hoy: `editor.rs` de 1.733 líneas con un único `struct Editor` de ~45 campos que hace interfaz,
lógica de edición, deshacer, guardado, captura de gráficos y lanzamiento del juego.

```
editor/src/
├─ domain/
│  ├─ document.rs     EditorDocument: ROM original + copia, 32 niveles (mapa, objetos),
│  │                  mapa del mundo, puntos, mapa propio, marcas de "sin guardar"
│  ├─ history.rs      History<T>: deshacer/rehacer genérico (reutilizado en niveles y mapa)
│  ├─ tools.rs        trait Tool { press, drag, release } → Pencil, BoxFill, Picker...
│  │                  (añadir herramientas = nueva impl, sin tocar el resto)
│  └─ space.rs        Cálculo de espacio libre por zona / objetos
├─ application/
│  ├─ commands.rs     PaintBlocks, FillBox, PlaceObject, MoveObject, DeleteObject,
│  │                  EditWorldTile, SetPointLevel, EditCustomMap...
│  ├─ save.rs         Guardar copia editada (usa cart-model::layout/patch)
│  ├─ art_service.rs  Captura de gráficos y sprites ejecutando la consola en segundo plano
│  └─ play.rs         Probar nivel (usa el puerto GameLauncher)
├─ ports.rs           RomRepository, EditedCopyRepository, GameLauncher, ArtCache
├─ ui/                macroquad, dividido por paneles
│  ├─ app.rs          Bucle y enrutado de entrada
│  ├─ level_view.rs   Vista del nivel, zoom, cámara, rejilla, vista general
│  ├─ palette.rs      Paleta de bloques y de tipos de objeto
│  ├─ world_view.rs   Editor del mapa del mundo (imagen y puntos)
│  ├─ custom_view.rs  "Tu mapa"
│  ├─ status_bar.rs
│  └─ widgets.rs      button, wrap, texturas
├─ adapters/
│  ├─ fs_repository.rs
│  └─ process_launcher.rs   Lanza game.exe con --level
└─ main.rs            Raíz de composición
```

## 6. Contrato entre juego y editor

Lo único que comparten fuera de los crates comunes (se mantiene **compatible** con lo actual,
para que partidas y ediciones existentes sigan sirviendo):

| Elemento | Formato |
|---|---|
| Lanzar el juego | `game.exe <rom> --level <0-31>` |
| Copia editada | `edits/<nombre> (edit).gb` |
| Mapa propio | `edits/<nombre> (edit).points.txt` |
| Partidas | `saves/<nombre>.sav` |
| Progreso | `saves/<nombre>.progress.txt` |
| Ajustes (idioma lo lee también el editor) | `settings.cfg` |

Las rutas y nombres viven en `host`, así ambos programas los calculan igual.

## 7. Estrategia de pruebas

1. **Huella dorada** (`regression/`): el test se porta a la API nueva y debe dar
   `golden.txt` idéntico. Se ejecuta en cada fase.
2. **Tests unitarios** del dominio puro: codecs RLE y de objetos, `History`, herramientas,
   máquina de estados del menú, navegación del mapa propio, progreso.
3. **Rendimiento**: frames por segundo del emulador nuevo vs. legacy (no debe empeorar).
4. **Manual / capturas**: menú y editor, comparando capturas con las del legacy.

## 8. Fases (cada una = commits pequeños con la huella en verde)

| Fase | Trabajo |
|---|---|
| 0 ✅ | Esqueleto del workspace; mover emulador y conocimiento casi tal cual a `gb-core` y `cart-model`; portar el test dorado |
| 1 ✅ | `gb-core`: encapsular campos, `Buttons`, `Screen`, `BusObserver` (sustituye al código de investigación del bus) |
| 2 ✅ | `cart-model`: `RomImage`, `ram_map`, `GameDriver`, errores tipados, separar ficheros |
| 3 | `host`: localizar ROM, rutas, ajustes |
| 4 | `game/`: dominio puro + puertos + adaptadores + raíz de composición |
| 5 | `editor/`: romper el objeto gigante en documento, historial, herramientas, comandos y paneles |
| 6 | `tools/research/` |
| 7 | Puntos de extensión: paletas desde fichero, paletas por capa, sustitución de gráficos, registro de ganchos |
| 8 | Paridad confirmada → borrar `legacy/` |

## 9. Decisiones abiertas

1. **`research.rs`** (971 líneas: desensamblador, vigilar/trazar memoria, capturas...):
   propuesta → conservarlo como `tools/research`, porque será útil para localizar gráficos y
   rutinas cuando hagamos mods.
2. **Nombres de los ejecutables**: propuesta → `game.exe` y `editor.exe` (hoy `gb_port.exe`).
   Las partidas no se pierden: dependen del nombre de la ROM, no del ejecutable.
