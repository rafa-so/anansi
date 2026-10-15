# anansi

Cliente HTTP desktop com as collections versionadas em git. Uma alternativa simples ao Postman para os requests do dia a dia: cada request é um arquivo JSON num repositório seu, então compartilhar com o time é fazer `git pull`.

O nome vem de Anansi, a aranha dos contos akan da África Ocidental que guarda todas as histórias do mundo.

- Stack: Tauri 2 + Svelte 5 + TypeScript. Toda a lógica fica no crate Rust `anansi-core` (sem dependência do Tauri); o crate do app só faz a ponte com a interface.

## Estado atual

O MVP está implementado: collections, pastas, CRUD e execução de requests, ambientes e segredos, importação e exportação de cURL e recarga automática quando os arquivos mudam por fora. O que só dá para validar no Windows está na seção "Validação no Windows", abaixo.

## Pré-requisitos (Windows)

| Ferramenta                     | Versão                                                       | Como instalar                                                                                        |
| ------------------------------ | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| Node                           | 22.18.0                                                      | `nvm-windows`: `nvm install 22.18.0` e `nvm use 22.18.0` (o `nvm-windows` não lê o `.nvmrc` sozinho) |
| Rust                           | stable, toolchain `x86_64-pc-windows-msvc`                   | <https://rustup.rs>                                                                                  |
| Visual Studio Build Tools 2022 | workload "Desktop development with C++" (MSVC e Windows SDK) | Instalador do Visual Studio                                                                          |
| WebView2                       | já incluso no Windows 10/11 atualizado                       | —                                                                                                    |
| Git for Windows                | 2.x                                                          | No clone do `minhas-collections`: `git config core.longpaths true`                                   |

Não é preciso rodar o terminal como Administrador. Se puder, exclua `src-tauri\target` da verificação em tempo real do Defender: o build Rust fica bem mais rápido.

## Desenvolvimento

```powershell
cd anansi
nvm use 22.18.0
npm install
npm run tauri dev      # abre o app com hot reload
```

A primeira compilação do Rust demora alguns minutos; as seguintes são incrementais.

## Testes

| Comando             | O que roda                                                                                                                         |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `npm run test:all`  | Checagem de tipos (`svelte-check`), testes do frontend (Vitest) e testes do `anansi-core` (`cargo test`). Rode antes de cada push. |
| `npm test`          | Só os testes do frontend                                                                                                           |
| `npm run test:rust` | Só os testes do `anansi-core` (unitários e de integração)                                                                          |
| `npm run test:perf` | Teste de desempenho: 500 requests carregados em menos de 1 s (build release)                                                       |
| `npm run test:e2e`  | Gera o binário de debug e roda os testes E2E no app real                                                                           |

Onde ficam:

- Rust: testes unitários no próprio arquivo (`#[cfg(test)] mod tests`) e de integração em `src-tauri/crates/anansi-core/tests/`. As fixtures ficam em `tests/fixtures/`; arquivos e pastas que começam com `_` viram `.` na cópia temporária (`_gitignore` → `.gitignore`), para não afetar o git deste repositório. Snapshots do `insta` em `tests/snapshots/` (para revisar mudanças: `cargo insta review`, com `cargo install cargo-insta`).
- Frontend: pasta `tests/` dentro da área que cobrem (ex.: `src/lib/state/tests/tabs.test.ts`). Testes que usam runes (`$state`) terminam em `.svelte.test.ts`.
- E2E: `e2e/specs/`.

### E2E no Windows

Uma vez por máquina:

1. `cargo install tauri-driver --locked`
2. Baixe o Microsoft Edge WebDriver (`msedgedriver.exe`) com a **mesma versão principal do WebView2** instalado (veja a versão em `edge://version` ou em "Aplicativos instalados" → "Microsoft Edge WebView2 Runtime") e coloque-o no `PATH`, ou informe o caminho em `ANANSI_E2E_NATIVE_DRIVER`.

Depois, `npm run test:e2e`. Os testes criam um workspace temporário a partir da fixture e um servidor HTTP falso local; nada sai da máquina. Quando o WebView2 atualizar, baixe o driver correspondente.

## Build

```powershell
npm run test:all
npm run tauri build
```

Saída: `src-tauri\target\release\bundle\nsis\anansi_<versão>_x64-setup.exe` (instalação por usuário, sem precisar de admin). Sem assinatura de código: o SmartScreen mostra "editor desconhecido" na instalação; clique em "Mais informações" → "Executar assim mesmo".

A versão fica em `src-tauri/tauri.conf.json` e no `package.json`; atualize as duas juntas.

## Uso básico

1. Abra a pasta do `minhas-collections` (na primeira vez, o app oferece inicializar o workspace).
2. "+ Collection" cria uma collection; o botão direito na árvore cria pastas e requests, renomeia, duplica, copia como cURL e remove.
3. **Ambientes…** (barra superior) cria os ambientes (`local`, `hml`, `prd`). Variáveis vão para `environments/<ambiente>.json` (versionado); segredos, como tokens e chaves de API, vão para `environments/<ambiente>.secrets.json`, que fica só na sua máquina. Escolha o ambiente ativo no seletor ao lado.
4. Use `{{nome}}` na URL, nos params, nos headers e no body. Nomes que não existem no ambiente ativo ficam em vermelho, e o app avisa antes de enviar.
5. `Ctrl+Enter` envia e `Esc` cancela. A resposta aparece embaixo (o divisor é arrastável); "Salvar…" grava o body completo em arquivo.
6. **Importar cURL** (rodapé da árvore) aceita o formato bash: "Copy as cURL (bash)" do Chrome/Edge ou o cURL do Postman.
7. Atalhos: `Ctrl+S` salva, `Ctrl+N` cria um request, `Ctrl+W` fecha a aba, `Ctrl+P` busca na árvore, `Ctrl+Tab` troca de aba.
8. Alterações feitas por fora (git pull, editor) aparecem sozinhas. Se o watcher não perceber (ex.: unidade de rede), use ⟳ (Recarregar workspace).
9. ⚙ (Preferências) define o padrão de verificação de TLS e o timeout. Cada request pode mudar isso na aba **Configurações**.

O body JSON ou texto de um request fica num arquivo ao lado dele (`criar-produto.json` + `criar-produto.body.json`), para o diff no git sair linha a linha.

"Copiar como cURL" tem duas variantes: com placeholders (`{{nome}}`), para compartilhar, e com valores, para rodar direto; esta pode conter segredos e o app avisa. O formato bash roda no Git Bash e no WSL; o PowerShell 7 usa `curl.exe` com crase de continuação (o Windows PowerShell 5.1 não é suportado, porque quebra as aspas do JSON).

## Validação no Windows

Os testes automatizados passaram num Linux (inclusive o E2E no app real), mas estes pontos só podem ser conferidos no Windows:

| Item                      | Como conferir                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------------- |
| Testes `#[cfg(windows)]`  | `npm run test:rust` (arquivo em uso por outro programa; letra de unidade minúscula)                 |
| Atalhos do WebView2       | `npm run test:e2e`: `Ctrl+P` precisa focar a busca sem abrir a impressão                            |
| Certificados corporativos | Chamar `hml` com "Verificar certificado TLS" ligado (usa o repositório de certificados do Windows)  |
| Proxy                     | Se a rede usar proxy, conferir uma chamada a `hml` (o proxy fixo do Windows é lido; script PAC não) |
| `localhost`               | Chamar um serviço rodando no Windows ou no Docker Desktop                                           |
| Instalador                | `npm run tauri build` e instalar o `.exe` gerado                                                    |

## Convenções

- Conventional Commits com escopo, em português depois do tipo: `feat(http): adicionar cancelamento de requisição`.
- Sem husky; rode `npm run test:all` antes do push.
- Scripts do `package.json` precisam funcionar no `cmd.exe` (é o shell que o npm usa no Windows).

## Licença

[MIT](LICENSE)
