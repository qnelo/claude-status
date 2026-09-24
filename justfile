name := 'claude-status'
desktop := 'io.github.qnelo.ClaudeStatus.desktop'
bin-dst := env('HOME') / '.local/bin' / name
desktop-dst := env('HOME') / '.local/share/applications' / desktop

run:
    RUST_BACKTRACE=full cargo run

check:
    cargo clippy --all-targets -- -W clippy::pedantic

# Instala en ~/.local, sin sudo. Exec lleva la ruta absoluta: el panel no depende del PATH.
install:
    cargo build --release
    install -Dm0755 target/release/{{name}} {{bin-dst}}
    mkdir -p "$(dirname {{desktop-dst}})"
    sed 's|^Exec=.*|Exec={{bin-dst}}|' res/{{desktop}} > {{desktop-dst}}

uninstall:
    rm -f {{bin-dst}} {{desktop-dst}}
