# Claude Status

A [COSMIC](https://system76.com/cosmic) panel applet that shows how much of your Claude subscription limits you have used, without opening claude.ai.

- **In the panel:** the current session usage (5-hour window). It turns yellow above 85 %. When the weekly limit goes past 90 %, it also shows up as `S N%`.
- **On click:** a popup with the current session and its countdown, the weekly limit across all models, and the per-model weekly limits, each with its reset time.

It refreshes every 5 minutes, or right away with the **Actualizar** button in the popup.

The interface is in Spanish.

## Requirements

- COSMIC desktop (Wayland).
- [Claude Code](https://claude.com/claude-code) logged in with your Claude account (Pro, Max, Team). The applet uses the token Claude Code stores in `~/.claude/.credentials.json`; it does not work with an API key.
- A recent stable Rust (edition 2024) and [`just`](https://github.com/casey/just).
- The system libraries libcosmic needs. On Pop!_OS / Ubuntu:

  ```sh
  sudo apt install just pkg-config libxkbcommon-dev libfontconfig-dev libfreetype-dev libexpat1-dev
  ```

## Install

```sh
git clone https://github.com/qnelo/claude-status.git
cd claude-status
just install
```

This builds in release mode and installs, without `sudo`:

- the binary to `~/.local/bin/claude-status`
- the `.desktop` file to `~/.local/share/applications/io.github.qnelo.ClaudeStatus.desktop`

Then add it to the panel: **Settings → Desktop → Panel → Applets → Add applet → Claude Status**. If it is not listed, restart the panel (see below).

## Update

```sh
git pull
just install
killall cosmic-panel
```

`killall cosmic-panel` restarts the panel and relaunches every applet (takes 5-10 s). Do not kill only `claude-status`: the panel will not bring it back.

## Uninstall

Remove it from the panel in Settings, then:

```sh
just uninstall
```

## Troubleshooting

The popup shows errors in its bottom-left corner. If a refresh fails, the last good data stays on screen.

| Message | What to do |
|---|---|
| `No pude leer ~/.claude/.credentials.json` | Log in to Claude Code (`claude`, then `/login`). |
| `Token vencido: abre Claude Code para renovarlo` | Open Claude Code once. The applet deliberately does not refresh the token: doing so would rotate the refresh token and log Claude Code out. |
| `La API pidió esperar` | Nothing; it retries on the next cycle. |
| The panel shows `–` | The first response has not arrived yet, or it failed. Open the popup to see the error. |

## How it works

It reads Claude Code's OAuth token (read-only) and calls `GET https://api.anthropic.com/api/oauth/usage`, the same endpoint claude.ai uses to show your limits. This is not a documented public API: if Anthropic changes it, the applet may stop working.

The token is only sent in that request, and error messages never show the contents of the credentials file.

## Development

```sh
just run      # run the applet outside the panel
just check    # clippy pedantic
cargo test    # response parsing and reset texts
```

- `src/main.rs`: the applet (panel view, popup, polling).
- `src/usage.rs`: token reading, API call, parsing and date formatting.
- `src/sample.json`: a real API response, used by the tests.
