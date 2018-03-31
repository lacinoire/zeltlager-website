# Zeltlager-Website

Website für Informationen und Anmeldung für das Zeltlager aus München.
Live findet ihr sie unter: [zeltlager.flakebi.de](https://zeltlager.flakebi.de)

# Setup

## Abhängigkeiten

- PostgreSQL
- Rust (kann mit [rustup](https://rustup.rs) installiert werden)
- Eventuell muss Rust zum Pfad hinzugefügt werden: `export PATH="$PATH:$HOME/.cargo/bin"`
- Diesel (installieren mit `cargo install diesel_cli`)

## Konfiguration

Es werden zwei Konfigurationsdateien benötigt:

`config.toml`:

```toml
# Von dieser E-Mail Addresse aus werden die Bestätigungsmails verschickt
email_username = "meine@email.addresse"
email_password = "Password für die E-Mail Addresse"

# Optional
# Standardwert: 127.0.0.1:8080
bind_address = "127.0.0.1:8080"
```

`.env`:

```sh
# Z. B. DATABASE_URL=postgres://zeltlager:meinpasswort@localhost/zeltlager
DATABASE_URL=postgres://<username>:<password>@<host>/<database>
```

## Starten

Beim ersten Mal muss `diesel setup` ausgeführt werden, nach Aktualisierungen des
Datenbankschemas reicht ein `diesel migration run` aus.

`cargo run --release`

Um Fehler zu finden:

`env RUST_BACKTRACE=1 RUST_LOG=actix_web=debug cargo run`

### Entwickler

Caro und Sebi, zwei Betreuer aus dem Zeltlager.
