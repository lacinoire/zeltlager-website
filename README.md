# Zeltlager-Website

Website für Informationen und Anmeldung für das Zeltlager aus München.
Live findet ihr sie unter: [meinzeltlager.com](https://meinzeltlager.com)

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
# Die maximale Anzahl an Teilnehmern
max_members = 70

# Wird bei manchen Fehlern angezeigt (zusätlich zu einer Fehlermeldung)
error_message = "Bitte informieren Sie uns über webmaster@meine.seite."

# Optional
# Standardwert: 127.0.0.1:8080
bind_address = "127.0.0.1:8080"

# Ob login-Cookies nur über https verwendbar sind
# Standardwert: true
secure = true

# Wird auf allen Seiten angezeigt, die mit dem basic-Template angezeigt werden.
global_message = "<h1>Ich bin ein globaler Header</h1>"

# Von dieser E-Mail Addresse aus werden die Bestätigungsmails verschickt
[sender_mail]
name = "Mein Name"
address = "mein.name@email.addresse"
[sender_mail_account]
host = "smtp.email.addresse"
name = "username"
password = "Password für die E-Mail Addresse"
```

`.env`:

```sh
# Z.B. DATABASE_URL=postgres://zeltlager:meinpasswort@localhost/zeltlager
DATABASE_URL=postgres://<username>:<password>@<host>/<database>
```

## Starten

```
cd frontend
bun install && bun run build
cd ..
cargo run --release
```

Um Fehler zu finden:

`env RUST_BACKTRACE=1 RUST_LOG=debug cargo run`

### Entwickler

Caro und Sebi, zwei Betreuer aus dem Zeltlager.
