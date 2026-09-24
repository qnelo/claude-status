# Claude Status

Applet para el panel de [COSMIC](https://system76.com/cosmic) que muestra cuánto llevas usado de los límites de tu suscripción de Claude, sin abrir claude.ai.

- **En el panel:** el porcentaje de la sesión actual (ventana de 5 horas). Se pone amarillo sobre 85 %. Cuando el límite semanal pasa de 90 %, aparece también como `S N%`.
- **Al hacer clic:** un popup con la sesión actual y su cuenta regresiva, el límite semanal de todos los modelos y los límites semanales por modelo, cada uno con su fecha de reinicio.

Se actualiza cada 5 minutos, o al instante con el botón **Actualizar** del popup.

## Requisitos

- Escritorio COSMIC (Wayland).
- [Claude Code](https://claude.com/claude-code) con la sesión iniciada con tu cuenta de Claude (Pro, Max, Team). El applet usa el token que Claude Code guarda en `~/.claude/.credentials.json`; con una API key no funciona.
- Rust estable reciente (edition 2024) y [`just`](https://github.com/casey/just).
- Las librerías de sistema que pide libcosmic. En Pop!_OS / Ubuntu:

  ```sh
  sudo apt install just pkg-config libxkbcommon-dev libfontconfig-dev libfreetype-dev libexpat1-dev
  ```

## Instalación

```sh
git clone git@github.com:qnelo/claude-status.git
cd claude-status
just install
```

Esto compila en modo release e instala, sin `sudo`:

- el binario en `~/.local/bin/claude-status`
- el `.desktop` en `~/.local/share/applications/io.github.qnelo.ClaudeStatus.desktop`

Luego agrégalo al panel: **Configuración → Escritorio → Panel → Applets → Agregar applet → Claude Status**. Si no aparece en la lista, reinicia el panel (ver abajo).

## Actualizar

```sh
git pull
just install
killall cosmic-panel
```

`killall cosmic-panel` reinicia el panel y relanza todos los applets (tarda 5-10 s). No mates solo `claude-status`: el panel no lo vuelve a levantar.

## Desinstalar

Quítalo del panel desde la configuración y luego:

```sh
just uninstall
```

## Problemas comunes

El popup muestra el error abajo a la izquierda. Si falla una actualización, se mantienen los últimos datos buenos.

| Mensaje | Qué hacer |
|---|---|
| `No pude leer ~/.claude/.credentials.json` | Inicia sesión en Claude Code (`claude` y luego `/login`). |
| `Token vencido: abre Claude Code para renovarlo` | Abre Claude Code una vez. El applet no renueva el token a propósito: hacerlo rotaría el refresh token y cerraría tu sesión de Claude Code. |
| `La API pidió esperar` | Nada; reintenta solo en el próximo ciclo. |
| El panel muestra `–` | Todavía no llega la primera respuesta, o falló. Abre el popup para ver el error. |

## Cómo funciona

Lee el token OAuth de Claude Code (solo lectura) y consulta `GET https://api.anthropic.com/api/oauth/usage`, el mismo endpoint que usa claude.ai para mostrar los límites. No es una API pública documentada: si Anthropic la cambia, el applet puede dejar de funcionar.

El token no sale de tu máquina más allá de esa llamada, y los errores nunca muestran el contenido del archivo de credenciales.

## Desarrollo

```sh
just run      # corre el applet fuera del panel
just check    # clippy pedantic
cargo test    # parseo de la respuesta y textos de reinicio
```

- `src/main.rs`: el applet (vista del panel, popup, polling).
- `src/usage.rs`: lectura del token, llamada a la API, parseo y formato de fechas.
- `src/sample.json`: respuesta real de la API, usada en los tests.
