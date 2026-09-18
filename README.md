# purrl

**Previously meow-cli.**

**Ask your terminal where a file went, in your own words.**

purrl embeds the *names* of your files with a local Ollama model, then ranks them
against a natural-language query. No cloud, no API keys, nothing leaves your machine.

```
purrl> ai find the qrcode I downloaded
purrl> ai find my logo image
purrl> open 1
```

---

## Requirements

purrl does nothing on its own. It talks to [Ollama](https://ollama.com) on
`http://localhost:11434`, which must be **installed and running**, with three models
pulled. This is the part worth reading before you clone anything.

| model | needed for | approximate size |
| --- | --- | --- |
| `nomic-embed-text` | indexing, and every search | 275 MB |
| `llama3:8b` | turning your sentence into a query | 4.7 GB |
| `llama3.2:3b` | breaking ties between close results | 2.0 GB |

```bash
ollama pull nomic-embed-text
ollama pull llama3:8b
ollama pull llama3.2:3b
```

That is roughly **7 GB of models on disk**. None of the three is optional: without
`llama3:8b` the `ai` command fails outright, and without `nomic-embed-text` neither
indexing nor searching works at all.

You also need **Rust 1.88 or newer** — edition 2024 sets a floor of 1.85, and one
dependency raises it to 1.88.

## Install

```bash
git clone https://github.com/pritam-ago/purrl.git
cd purrl
cargo build --release
```

The binary is `purrl`. Run it with `cargo run`, or install it with
`cargo install --path .`.

## Usage

```
purrl> index
purrl> ai find logo image
purrl> open 1
```

Inside the shell:

| command | what it does |
| --- | --- |
| `index` / `reindex` | build the semantic index |
| `ai <query>` | search using natural language |
| `open <n>` | open result number `n` |
| `clear` | clear the terminal |
| `exit` / `quit` | leave |

From the command line:

| command | what it does |
| --- | --- |
| `purrl` | start the shell (same as `purrl shell`) |
| `purrl index` | build the index and exit |
| `purrl --db <PATH>` | use a different index file |
| `purrl hi` | check it runs |

## What gets indexed

Two directories, both hardcoded: `~/Downloads` and `~/OneDrive/Pictures`. Making
them configurable is on the roadmap.

**Only the filename, the extension and the parent folder name are embedded. File
contents are never read.** purrl builds a short sentence per file — "This is a pdf
file named hostel fees located in Downloads folder" — and embeds that. So it finds
files *named* like your query. It cannot find a file by something written inside it,
and nothing in the index would let it.

## Where the index lives

`<data_dir>/purrl/vectors.db`:

| platform | path |
| --- | --- |
| Windows | `%APPDATA%\purrl\vectors.db` |
| macOS | `~/Library/Application Support/purrl/vectors.db` |
| Linux | `~/.local/share/purrl/vectors.db` |

Pass `--db <PATH>` to point somewhere else.

Versions before the rename wrote `meow_vectors.db` into whatever directory you
happened to be standing in, which is an index of your personal file names sitting in
your working tree. If you have one, purrl says so once on startup; it does not read
it, migrate it or delete it. Deleting it is safe.

## How it works

1. `index` walks the two directories above.
2. Each file becomes a one-sentence description built from its name, extension and
   folder, which `nomic-embed-text` turns into a vector in SQLite.
3. `ai <query>` sends your sentence to `llama3:8b`, which extracts a query and
   optional folder and time hints.
4. The query is embedded and compared against every stored vector by cosine
   similarity. The top ten are shown, best first.
5. If the top two results are close and neither is confident, `llama3.2:3b` gets a
   second look and may promote one to the top.

## What purrl does not do

- **Read file contents.** See above.
- **Delete anything.** A search tool does not need to remove files, and it certainly
  does not need to do so on a language model's say-so. Asking it to delete something
  gets a refusal, not a deletion. This is not on the roadmap.
- **Learn from what you pick.** Selection-history ranking and a filename-match
  ranking term were both described as working in earlier versions of this README.
  Neither was ever written. They are on the roadmap, unbuilt.

## Roadmap

See [ROADMAP.md](ROADMAP.md).

## Development

```bash
cargo test                                # no Ollama needed
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

The tests cover the pure logic — similarity, ranking, storage, path handling — and
never contact Ollama, so they run anywhere. CI runs all four commands on Ubuntu, plus
a `cargo check` on Rust 1.88 to keep the version above honest.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License
