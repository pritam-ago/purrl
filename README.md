# purrl

**Previously meow-cli.**

**Ask your terminal questions about your files.**

`purrl` is a local-first, AI-powered command-line tool that understands your filesystem.
Instead of remembering filenames or paths, you can search using natural language.

```bash
purrl> ai find the qrcode I downloaded
purrl> ai find my logo image
purrl> ai find ubuntu iso
purrl> open 1
```

No cloud. No API keys. Fully local.

---

## Features

- Semantic search over your files (meaning > filenames)
- Hybrid ranking (semantic + filename + learning)
- Interactive CLI with selection & open
- Implicit learning (purrl learns what you pick)
- Privacy-first (runs locally with Ollama)
- Fast & lightweight (Rust)

---

## Installation

### Prerequisites
- Rust (latest stable)
- Ollama running locally
- Ollama embedding model:
```bash
ollama pull nomic-embed-text
```

### Build from source
```bash
git clone https://github.com/pritam-ago/meow-cli.git
cd meow-cli
cargo build --release
```

Run:
```bash
cargo run
```

---

## 🧪 Usage

```bash
purrl> index
purrl> ai find logo image
purrl> open 1
```

Commands:
- `index` – build semantic index
- `ai <query>` – search using natural language
- `open <n>` – open result by number
- `clear` – clear terminal
- `exit` – quit

---

## How it works

1. Indexes your Downloads & Pictures
2. Builds embeddings using Ollama
3. Converts your query to embeddings
4. Ranks files by meaning + filename + learning
5. Shows the most relevant results

---

## Roadmap

See [ROADMAP.md](ROADMAP.md)

---

## Contributing

Contributions are welcome!
Please read [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

MIT License
