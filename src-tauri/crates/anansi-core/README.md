# anansi-core

## Introdução

Toda a lógica do `anansi`: leitura e escrita do workspace (collections, pastas, requests e bodies), slugs, validação de caminhos, ambientes e segredos, resolução de `{{variáveis}}`, execução HTTP, cURL (exportar e importar) e file watcher. Não depende do Tauri: o crate do app (`src-tauri/src`) só converte os comandos do frontend em chamadas daqui.

## Dependências

| Crate | Uso |
|---|---|
| `serde`, `serde_json` | Formato dos arquivos e do IPC |
| `thiserror` | `AppError` |
| `deunicode` | Slugs sem acento e ordenação da árvore |
| `dunce` | Caminhos no Windows sem o prefixo `\\?\` |
| `ignore` | Checagem do `.gitignore` |
| `reqwest` (rustls com raízes do sistema), `tokio`, `url` | Execução HTTP |
| `shell-words`, `base64` | Importar cURL |
| `notify`, `notify-debouncer-full` | File watcher |
| `tempfile`, `rstest`, `insta`, `pretty_assertions`, `wiremock`, `rcgen`, `tokio-rustls`, `flate2` (dev) | Testes (servidores HTTP e HTTPS falsos) |

## Documentação

- Formato dos arquivos e uso: `README.md` na raiz do repositório.
- Testes: `cargo test` nesta pasta, ou `npm run test:rust` na raiz do `anansi`.
