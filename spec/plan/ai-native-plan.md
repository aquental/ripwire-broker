# Plano — prontidão para agentes (AI-native)

Data: **2026-11-14**. Base: `master`, HEAD `d9aa86e`.
Origem: [`docs/AINE-REPORT.md`](../../docs/AINE-REPORT.md) (26 aprovados, 20 reprovados, 0 não
aplicáveis).
Status: **planejado; nada implementado**. Revisado pelo Codex em 2026-11-14: 14 achados, todos
conferidos no código e incorporados (ver [Revisão adversarial](#revisão-adversarial)).
Decisão: [D-189](../changelog.md#d-189--plano-de-prontidão-para-agentes-revisado-pelo-codex).

## Objetivo e limites

Fechar os 20 itens reprovados do relatório. Primeiro os de prioridade alta (Fase A, etapas 1 a 6),
depois os de prioridade baixa (Fase B, etapas 7 a 10). Sete dos vinte são o mesmo defeito: não
existe um arquivo de instruções que o agente carregue no início da sessão.

Este plano não altera código de produto (`src/`), o contrato MCP, dependências Cargo nem os nomes
dos jobs obrigatórios `default` e `online` (D-107). Toda etapa que muda configuração do GitHub ou
credenciais (etapas 3, 4 e 5) é executada pelo mantenedor, nunca por um agente. Cada decisão
tomada na execução vira uma entrada `D-NNN` em [`spec/changelog.md`](../changelog.md).

## Mapa: item do relatório → etapa

| Item reprovado no relatório | Prioridade | Etapa |
| --- | --- | --- |
| Arquivo de instruções; comandos; nomes que existem; especificidade; layout | alta | 1 |
| Regras não óbvias; atualidade; tamanho do arquivo de instruções | baixa | 1 |
| Operações que precisam de um humano | alta | 2 |
| Varredura automática de segredos | alta | 3 |
| Alcance das credenciais do agente | alta | 4 (risco aceito; segue reprovado) |
| Humano no caminho de toda ação que muda produção | alta | 5 (parcial; segue reprovado, ver etapa 4) |
| Scripts de tarefa; scripts nomeados; um comando que roda tudo | alta | 6 |
| Regras de revisão escritas | baixa | 7 |
| Template de spec | baixa | 8 |
| Caminho de volta (rollback) | baixa | 9 |
| Biblioteca de skills; cobertura do trabalho repetido | baixa | 10 |

## Ordem de execução

6 → 1, 2 e 4 juntas → 3 → 5 → 7 → 8 → 9 → 10.

- A etapa 6 vem primeiro: o `AGENTS.md` nomeia o comando que ela cria, e não pode ser aceito
  citando um comando que ainda não existe.
- As etapas 1 e 2 saem na mesma PR: a lista da 2 é uma seção do arquivo da 1.
- A etapa 4 é só documentação (risco aceito, D-192) e sai junto com a 1 e a 2.
- A etapa 3 acrescenta um verificador ao comando da etapa 6; ela fecha o comando único.

## Fase A — prioridade alta

### 1. `AGENTS.md` na raiz, como roteador

**Arquivos:** `AGENTS.md` (novo), `CLAUDE.md` (novo, só a linha `@AGENTS.md`), `handoff.md`.

- [ ] Criar `AGENTS.md` com no máximo ~1.500 tokens (6 KB), em inglês (convenção: código, commits
  e PRs em inglês; `spec/` em português). Conteúdo próprio, sem copiar o `handoff.md`:
  - o que é o projeto, em três linhas;
  - os comandos de verificação (o script da etapa 6 e seus subcomandos);
  - ponteiros nomeando o alvo e o que há nele: `handoff.md` §"Como verificar", §"Onde está o
    quê", §"Armadilhas já pisadas", §"Convenções"; `spec/changelog.md` (decisões, D-NNN);
    `spec/ripwire-broker-mcp.md` (PRD, CA-01 a CA-10); `README.md` §Tests;
  - as cinco regras que mais custam quando quebradas: entrada D-NNN por decisão; nenhum crate de
    rede no build padrão (CA-10); não renomear os jobs `default`/`online`; TDD pela costura
    pública em `tests/`; fixtures sempre sintéticas;
  - a lista "Ask before" da etapa 2.
- [ ] `CLAUDE.md` com `@AGENTS.md`, para o Claude Code carregar o mesmo texto.
- [ ] Declarar no `AGENTS.md` que `integrations/codex/AGENTS.md` é conteúdo de produto (o que o
  `install codex` entrega ao usuário), não instrução para trabalhar neste repositório.
- [ ] Reconciliar o `handoff.md` consigo mesmo. O cabeçalho diz "Estado em 2026-10-10, até o
  D-169", mas a seção da T5.3 (linha 160) e as pendências (linha 326) já trazem o D-185 ao
  D-187; só o D-188 falta. Atualizar o cabeçalho, a tabela de estado e §"Como verificar" (o CI
  instala o ripwire desde o D-188) para um estado único e datado. Uma frase genérica "o
  changelog prevalece" não basta: deixaria instruções operacionais antigas no caminho do agente.
- [ ] Conferir cada caminho e cada comando citado contra a árvore antes de entregar.

**Aceitação:** `wc -c AGENTS.md` ≤ 6.000; todo caminho citado existe e todo comando citado
roda; `scripts/check.sh all` verde (inclui `tests/plugin.rs` e os dois
`claude plugin validate --strict`: a raiz é o marketplace `aquental`, e um arquivo novo nela não
pode quebrar isso). **Dependências:** etapa 6 concluída; etapa 2 na mesma PR. **Risco:** o
`handoff.md` é em português e o `AGENTS.md` em inglês; os ponteiros citam os títulos das seções
como estão.

### 2. Lista "Ask before"

**Arquivos:** `AGENTS.md`; uma frase de aviso no `README.md` junto de `memory forget`.

As regras são escritas pelo efeito, não pela flag: uma flag nova com o mesmo efeito já está
coberta.

- [ ] Seção "Ask before" no `AGENTS.md`, com o motivo de cada linha:
  - **publicar**: criar ou empurrar uma tag `v*`, criar ou editar um release, `gh pr merge`,
    `git push` para o `master`;
  - **mudar o que o usuário do plugin executa**: qualquer edição em `integrations/claude-code/`
    (o `checksums.txt`, mas também `hooks/register.ts`, `scripts/broker`, `.mcp.json`,
    `install-binary.sh`) ou em `.claude-plugin/marketplace.json`. O marketplace entrega o que
    está no `master`, com as permissões do usuário;
  - **gastar dinheiro ou enviar conteúdo a um provider**: qualquer execução que fale com o
    classificador. Isso inclui `--online`, e também `serve --memory` (o `scripts/broker`, linha
    104: "--memory implies online on the server"), as opções `online`/`memory` do plugin,
    `memory drain --online`, `doctor --jev-probe`, os testes `--ignored` de
    `tests/online_live.rs`, e as rodadas do `ripwire-eval` com agentes pagos (`handoff.md`,
    "piloto pago");
  - **escrever fora do repositório**: `install … --write` fora de um diretório temporário;
  - **apagar estado real**: `memory forget --all` em um state dir que não seja de teste;
  - **mexer nos portões**: renomear jobs do `rust.yml`, proteção de branch, rulesets,
    environments, a allowlist do gitleaks, o `deny.toml`.
- [ ] No `README.md`, marcar `memory forget --all` como irreversível.

**Aceitação:** as seis categorias existem, cada uma com o motivo e os exemplos; a lista está na
primeira tela do `AGENTS.md`. **Dependências:** mesma PR da etapa 1.

### 3. Varredura de segredos

**Arquivos:** `.github/workflows/rust.yml`, `.gitleaks.toml` (novo), `scripts/check.sh`,
configuração do repositório.

- [ ] Mantenedor: ligar no GitHub `secret_scanning` e `secret_scanning_push_protection` (o
  repositório é público, então é gratuito). Conferir com
  `gh api repos/aquental/ripwire-broker --jq .security_and_analysis`.
- [ ] Novo job `secrets` no `rust.yml`, em `pull_request` e `push` ao `master`, **não
  obrigatório** no início: baixa o binário do gitleaks de uma release fixada, confere o SHA-256
  (mesmo padrão do passo do ripwire, sem action de terceiros) e roda
  `gitleaks detect --redact` sobre o histórico (checkout com `fetch-depth: 0`).
- [ ] `.gitleaks.toml` com allowlist por caminho e linha para os stubs sintéticos conhecidos
  (`tests/online_units.rs`, `tests/props_fs.rs`: cabeçalhos `BEGIN … PRIVATE KEY` escritos em
  tempdir para testar o leitor), cada entrada com o motivo.
- [ ] Rodar o gitleaks localmente sobre o histórico inteiro antes de abrir a PR e tratar o que
  aparecer.
- [ ] `scripts/check.sh secrets`, incluído no `all` (etapa 6).
- [ ] Manter o passo "Fixtures stay synthetic": ele cobre padrões (`authorization`, `api_key`)
  que o gitleaks não trata como segredo.

**Aceitação**, em dois ensaios separados, porque um push recusado nunca chega a virar PR:

1. *gitleaks:* numa cópia local, um arquivo novo com uma amostra sintética que o gitleaks
   detecta (escolhida rodando o próprio gitleaks; registrar qual regra disparou) faz
   `scripts/check.sh secrets` sair ≠ 0; sem o arquivo, sai 0 no `master`. O job do CI é
   conferido com a mesma amostra num branch descartável, com o push protection ainda desligado
   ou com a amostra num formato que só o gitleaks pega.
2. *push protection:* com o recurso ligado, empurrar para um branch descartável uma amostra de
   um tipo que o GitHub documenta como bloqueado no push é recusado. Uma chave AWS isolada não
   serve: o GitHub casa o par ID e segredo no mesmo arquivo. Escolher o tipo pela lista de
   padrões suportados e registrar qual foi usado.

**Risco:** falso positivo nos testes que escrevem chaves falsas de propósito; por isso a
allowlist e o início como não obrigatório. Promover a obrigatório é uma decisão (D-NNN) depois
de duas semanas sem falso positivo.

### 4. Credencial do agente: risco aceito

**Arquivos:** `AGENTS.md` (uma seção curta), `handoff.md` §Convenções.

**Decisão do mantenedor em 2026-11-14
([D-192](../changelog.md#d-192--credencial-do-agente-risco-aceito)):** as sessões de agente
continuam rodando no usuário do mantenedor, com o login do GitHub dele (keyring, escopo `repo`)
ao alcance. Nenhum isolamento, conta de máquina ou token restrito será montado. As opções A
(conta de máquina com fork, D-190) e B (nenhuma credencial na sessão, D-191) ficam descartadas.

O que a revisão mostrou, e por que meias medidas não entram:

- para abrir PRs é preciso `contents:write`, e essa permissão também cria releases pela API e
  empurra tags;
- `GH_TOKEN` só tem precedência sobre o login guardado; removida a variável, o `gh` volta ao
  keyring;
- para o GitHub, um PAT do dono é o dono.

Um token restrito sem isolamento daria a aparência de proteção sem a proteção; por isso a
decisão é entre isolar e aceitar o risco, e o mantenedor aceitou.

**O risco, dito por inteiro.** Um agente nesta máquina, ou uma injeção de prompt que ele
obedeça, consegue sem nenhum humano no caminho: empurrar uma tag `v*`, criar um release pela
API, mesclar uma PR (o `master` exige os checks e nenhuma revisão), aprovar o environment
`release` da etapa 5, e mudar o plugin que o marketplace entrega do `master`.

**O que sobra como proteção**, toda ela dependente de o agente obedecer:

- [ ] a lista "Ask before" da etapa 2, que nomeia exatamente essas ações;
- [ ] uma seção "Credentials" no `AGENTS.md`: a sessão tem o login do mantenedor; `git push`
  para o `master`, tags, releases, `gh pr merge` e aprovações de environment são sempre do
  mantenedor, mesmo que o comando funcione;
- [ ] os modos de permissão do host (confirmação antes de `git push` e de `gh` que escreve),
  quando o mantenedor os usar.

**Aceitação:** a seção existe no `AGENTS.md`; o D-192 registra o risco. **Consequência no
relatório:** os itens "alcance das credenciais do agente" e "humano no caminho de toda ação que
muda produção" **continuam reprovados** numa nova auditoria. Isso é esperado, não um defeito
da execução do plano. **Reabrir se:** entrar um segundo mantenedor, o plugin ganhar usuários
fora do mantenedor em número relevante, ou as sessões passarem a rodar sem supervisão (loops,
agentes agendados).

### 5. Um humano antes de publicar

**Arquivos:** `.github/workflows/release.yml`, `.github/workflows/rust.yml`, `tests/plugin.rs`,
configuração do repositório, `integrations/claude-code/README.md`.

Há três rotas até o usuário: a tag `v*` (publica o release), a API de releases, e o `master`
(o marketplace entrega o plugin do `master`: `register.ts`, `scripts/broker`, `.mcp.json`, o
instalador e o `checksums.txt`). As três estão ao alcance de quem tem o login do mantenedor, e pela etapa 4 (risco aceito) isso
inclui as sessões de agente. Esta etapa não fecha o item do relatório: ela barra o acidente (uma
tag empurrada por engano, um hash errado) e torna a superfície do plugin visível na PR. Não
barra um agente que decida, ou seja induzido a, aprovar e publicar.

- [ ] Mantenedor: criar o environment `release`, restrito a tags `v*`, com a configuração
  escrita aqui para não ficar implícita:
  - revisor obrigatório: o mantenedor;
  - "prevent self-review" **desligado**: o mantenedor é quem empurra a tag e é o único revisor;
    com a opção ligada ninguém poderia aprovar;
  - bypass de administradores **desligado**.
  Com o login do mantenedor ao alcance do agente (etapa 4), essa aprovação é um segundo passo
  deliberado, não uma garantia de presença humana.
- [ ] `release.yml`: `environment: release` no job `draft`, o primeiro com `contents: write`.
  O job `gates` (que chama o `rust.yml` e compila para rodar clippy e os testes) continua
  antes, sem aprovação. A aprovação antecede a criação do rascunho, a compilação dos artefatos
  distribuídos e a publicação. Manter a ordem gates → draft → build → publish.
- [ ] `tests/plugin.rs` lê o `release.yml`
  (`the_release_workflow_publishes_what_install_binary_downloads`): rodar e ajustar se a chave
  nova quebrar a leitura; acrescentar uma asserção de que o job `draft` declara
  `environment: release`, para o portão não sumir numa edição.
- [ ] Verificação do `checksums.txt` por máquina, como defesa adicional e não como substituta
  da revisão: um passo no job `default` que, quando o arquivo muda, baixa o `SHA256SUMS` do
  release da tag da primeira linha e compara linha a linha. Um hash que não veio de um release
  publicado falha.
- [ ] Um passo que escreve um aviso no resumo do job quando a PR toca `integrations/claude-code/`
  ou `.claude-plugin/`: "esta PR muda o que os usuários do plugin executam". Informativo, não
  bloqueia.
- [ ] Documentar o portão em "Publishing a version" no README do plugin.

**Aceitação:**

- no próximo release real (a tag precisa ser a versão do `Cargo.toml`; uma tag de teste com
  outra versão é recusada pelo passo "The tag names the crate's version" e não provaria nada):
  `gh run view --json jobs` mostra `gates` concluído e `draft` em `waiting`, nenhum job
  posterior iniciado e `gh release view <tag>` sem rascunho, até a aprovação na interface;
- `gh api repos/aquental/ripwire-broker/environments/release` mostra o revisor, o
  self-review permitido e o bypass de administradores desligado;
- uma PR que troca um hash do `checksums.txt` falha no `default`;
- `scripts/check.sh all` verde.

**Risco:** o passo dos hashes faz rede no job `default` (só `gh release download` de um
repositório público, só quando o arquivo muda). **Decisão em aberto:** exigir revisão aprovada
no `master`. Não entra agora: com um só mantenedor e `enforce_admins` ligado, travaria as PRs
do próprio dono. Volta à mesa se houver um segundo mantenedor.

### 6. Um comando que roda tudo

**Arquivos:** `scripts/check.sh` (novo), `.cargo/config.toml` (novo), `rust.yml`, `README.md`
§Tests, `handoff.md` §"Como verificar".

- [ ] `scripts/check.sh [alvo]` (bash, `set -euo pipefail`), com os mesmos passos e a mesma
  ordem do `rust.yml`:
  - `default`: `cargo fmt --all --check`; clippy `-D warnings`; `cargo nextest run --locked`;
    `props` com `PROPTEST_CASES=4096`; `props_fs`; o portão do CA-10; a guarda de fixtures;
  - `online`: clippy e nextest com `--features online`;
  - `plugin`: os dois `claude plugin validate --strict` e o `claude plugin test`;
  - `secrets`: o gitleaks (entra na etapa 3);
  - `all` (padrão): **os quatro**. É o que "tudo" quer dizer: tudo o que o CI roda numa PR.
- [ ] `all` falha, com a instrução de instalação, quando falta uma ferramenta (`cargo-nextest`,
  que hoje não está instalado na máquina do mantenedor; `claude` na versão fixada no
  `rust.yml`; `gitleaks`). Não cai para `cargo test` nem pula alvos em silêncio.
- [ ] Rodar um alvo isolado é permitido e termina com uma linha explícita: "cobertura parcial:
  rodou X; não rodou Y, Z". Só `all` imprime "todas as verificações de PR passaram".
- [ ] Avisar quando o `ripwire` não está no `PATH`: sem ele os testes e2e retornam cedo e passam
  sem testar nada (`require_ripwire!`). Em `all`, isso também é falha.
- [ ] Fora do `all`, de propósito: `deny` (`cargo deny --all-features check`, que segue
  agendado e fora das PRs, D-108) e `live` (testes pagos; lista da etapa 2).
- [ ] `rust.yml`: os jobs `default`, `online` e `plugin` passam a chamar
  `scripts/check.sh default|online|plugin`. Os nomes dos jobs não mudam. O timeout de 12 min
  do passo de testes (D-144) vira o timeout do passo único, menor que o do job, para o log
  sobreviver. O script imprime `::group::` por verificação, para o log manter a granularidade.
- [ ] `.cargo/config.toml` só com `[alias]`: `lint` e `lint-online` (os dois clippy). O que
  precisa de variável de ambiente fica no script.
- [ ] Atualizar README e handoff para nomear o script (o `AGENTS.md` nasce já com ele).

**Aceitação:** `scripts/check.sh` sai 0 numa árvore limpa e ≠ 0 em cada uma destas mutações,
com o diagnóstico esperado:

- um `println!("x");` compilável dentro de uma função da biblioteca (por exemplo em
  `src/budget.rs`): falha com `clippy::print_stdout`. Não vale em `src/main.rs`, que é outra
  raiz e imprime legitimamente (`src/lib.rs`, linhas 5 a 9), nem um erro de sintaxe;
- uma asserção invertida num teste: falha no nextest, com o nome do teste;
- `reqwest` como dependência não opcional: falha no portão do CA-10;
- `claude` fora do `PATH`: `all` falha dizendo o que instalar; `default` passa e imprime
  "cobertura parcial".

E ainda: o CI verde chamando o script, com os três nomes de job intactos; `cargo lint`
funciona; `cargo build --locked` produz o mesmo resultado com e sem `.cargo/config.toml`.

## Fase B — prioridade baixa

### 7. Template de PR

**Arquivos:** `.github/pull_request_template.md` (novo).

- [ ] Checklist curto, tirado do `handoff.md` §Convenções: teste vermelho antes; entrada D-NNN
  se houve decisão; nenhum crate novo no grafo padrão; `scripts/check.sh all` verde; fixtures
  sintéticas; "esta PR muda o que o usuário do plugin executa? (`integrations/claude-code/`,
  `.claude-plugin/`)"; o que foi verificado e como.

**Aceitação:** uma PR nova abre com o checklist. As PRs do Dependabot não são afetadas.

### 8. Template de spec

**Arquivos:** `spec/TEMPLATE.md` (novo); uma linha em `handoff.md` §Convenções.

- [ ] Forma que os PRDs já usam: contexto e problema; objetivos e não objetivos; critérios de
  aceitação em Dado/quando/então, numerados; falhas e comportamento obrigatório; como desfazer;
  decisões D-NNN relacionadas; status e data.
- [ ] Não reformatar os documentos existentes.

**Aceitação:** o arquivo existe e a convenção aponta para ele.

### 9. Caminho de volta documentado

**Arquivos:** `integrations/claude-code/README.md`.

- [ ] Subseção "Rolling back a version" depois de "Publishing a version", com a sequência do
  lado do mantenedor e do lado do usuário:
  - mantenedor: reverter o `checksums.txt` e a versão do `plugin.json` para a tag anterior
    (`tests/plugin.rs` exige `plugin_version_equals_the_pinned_release` e
    `cargo_version_is_not_behind_the_plugin_version`); marcar o release ruim como pre-release;
  - usuário: `claude plugin update ripwire-broker@aquental`, rodar o `install-binary.sh` de
    novo, reiniciar a sessão;
  - o que **não** volta sozinho: quem configurou a opção `binary` do plugin, ou tem outro
    `ripwire-broker` no `PATH`, continua no binário escolhido (`scripts/broker`, linha 39: a
    opção vence o pin). Dizer como conferir qual executável está em uso (`doctor`).
- [ ] Store de memória com esquema desconhecido. `memory forget --all` **não** é a saída:
  `forget_everything` (`src/memory/store.rs`, linha 773) revoga a coleta e apaga as pendências,
  mas depois chama `self.load()`, que devolve `UnknownSchema`; o comando falha e o snapshot
  fica. Documentar o que funciona: usar um binário que entenda o esquema; ou, com o servidor e
  os hooks parados, mover o diretório do store para fora do state dir (backup) e recomeçar.
  Hoje só existe `SCHEMA_VERSION = 1`, então isso é preparo para a primeira mudança de esquema,
  não um incidente possível agora.
- [ ] Se o ensaio mostrar que falta um comando para descartar um store ilegível, abrir issue;
  é mudança de produto e fica fora deste plano.

**Aceitação:** ensaio de ponta a ponta numa instalação real do plugin, sem publicar nada:
instalar a versão atual, aplicar o rollback num marketplace local (`--plugin-dir` ou um clone),
atualizar, reinstalar o binário, reiniciar e conferir pelo `doctor` qual executável roda, com e
sem a opção `binary`. Ensaio do esquema: um store com `schema_version` adulterado para 2 num
state dir temporário; conferir que o broker recusa sem corromper e que o procedimento
documentado recupera. `scripts/check.sh all` verde no branch do rollback.

### 10. Skills versionadas

**Arquivos:** `.claude/skills/` (versionar), `skills-lock.json`, `.gitignore`.

- [ ] Decidir o que fazer com as três cópias não versionadas do `aine-readiness-analyzer`
  (`.claude/`, `.agents/`, `.grok/`) e o `skills-lock.json`. Proposta: versionar
  `.claude/skills/` e o lock; ignorar `.agents/` e `.grok/` no `.gitignore`.
- [ ] `decision-entry`: acrescenta a linha no índice e a seção `## D-NNN` em
  `spec/changelog.md`, com a data de `date`. Antes, acertar o índice: o D-188 não tem linha
  nele, e o bloco do D-182 ao D-187 está em ordem decrescente dentro do trecho que a convenção
  diz ser crescente.
- [ ] `release`: a sequência de "Publishing a version". A skill prepara (versão, gates, texto
  da PR do `checksums.txt` a partir do `SHA256SUMS` publicado) e **para** nos passos que são do
  mantenedor: a tag, a aprovação do environment, a mescla. Ela teria como fazê-los (etapa 4), então a parada
  é regra da skill e é conferida na revisão dela.
- [ ] `audit-rust`: `spec/prompt/auditoria-rust.md` como skill invocável; o arquivo original
  vira um ponteiro, para não haver duas cópias.
- [ ] Nenhuma skill executa algo da lista "Ask before" sem parar para perguntar; conferir cada
  uma contra a lista, pelo efeito.

**Aceitação:** um clone novo tem as skills; `decision-entry` produz uma entrada que segue o
formato das existentes. **Dependências:** etapas 1, 2, 4 e 5.

## Fora do relatório, achado na auditoria

- [ ] Um teste de `tests/plugin.rs` falhou uma vez em `cargo test --locked` e passou nas duas
  execuções seguintes (886 aprovados no total). O nome não foi capturado. Reproduzir com
  `cargo nextest run --test plugin` em laço sob carga, abrir issue com o nome, e tratar como os
  #120 e #122.

## Verificação final

- [ ] Rodar de novo o analisador e comparar com o `docs/AINE-REPORT.md` de 2026-11-14.
  Esperado: 18 dos 20 reprovados passam. Os itens de credenciais e de humano no
  caminho continuam reprovados, por decisão (D-192).
- [ ] Entradas D-NNN para: o `AGENTS.md` como ponto de entrada; o environment `release` e sua configuração; a verificação do
  `checksums.txt` por máquina; o script único como fonte dos passos do CI; o gitleaks e a
  promoção a obrigatório.

## Revisão adversarial

Feita pelo Codex (`codex exec --sandbox read-only`, codex-cli 0.169.0) em 2026-11-14, sobre a
primeira versão deste plano. O Codex não conseguiu consultar a API do GitHub (sem rede no
sandbox); o estado das proteções citado aqui vem das consultas feitas na auditoria. Os 14
achados foram conferidos um a um no código; todos se aplicam.

| # | Sev. | Achado | Conferido em | O que mudou |
| --- | --- | --- | --- | --- |
| 1 | alta | `contents:write` cria releases pela API, sem passar pelo workflow | permissão da API de releases; etapa 4 original | Etapa 4 reescrita: nenhuma credencial com escrita no repositório principal na sessão do agente (conta de máquina com fork, ou nenhuma credencial); teste de publicação direta na aceitação |
| 2 | alta | `GH_TOKEN` só tem precedência; removê-lo volta ao login do keyring | `gh auth status` mostra o login por keyring com escopo `repo` | Isolamento do ambiente do agente passa a ser requisito, com aceitação própria |
| 3 | alta | Comparar hashes não cobre o resto do plugin, que o marketplace entrega do `master` | `integrations/claude-code/README.md` linhas 121–137 e 157 | A mescla pelo mantenedor vira o portão de toda a superfície distribuída; a comparação de hashes fica como defesa adicional; aviso na PR; categoria própria na lista "Ask before" |
| 4 | alta | Environment sem definir autoaprovação e bypass | etapa 5 original | Configuração escrita: self-review permitido, bypass de administradores desligado, e a dependência da etapa 4 dita com todas as letras |
| 5 | alta | `memory forget --all` falha num store de esquema desconhecido | `src/memory/store.rs` linhas 773–784 e 1341 | Etapa 9 corrigida: recuperação por binário compatível ou por mover o store com os processos parados; ensaio do cenário |
| 6 | média | Ensaio em branch não prova rollback para quem já instalou | `README` do plugin linha 157; `scripts/broker` linha 39 | Aceitação da etapa 9 passa a ser de ponta a ponta, cobrindo a opção `binary` e o `PATH` |
| 7 | média | O comando "que roda tudo" deixava o plugin e o gitleaks de fora | `rust.yml`, job `plugin` | `all` roda os quatro alvos e falha se faltar ferramenta; alvo isolado declara cobertura parcial |
| 8 | média | A lista de operações pagas não cobria `--memory` nem as avaliações | `scripts/broker` linha 104; `handoff.md` linha 138 | Lista reescrita pelo efeito, com `serve --memory`, opções do plugin, `--jev-probe` e o `ripwire-eval` pago |
| 9 | média | Um push recusado nunca vira PR; chave AWS isolada pode não ser detectada | lógica do próprio critério | Dois ensaios separados na etapa 3, cada um com amostra comprovada |
| 10 | média | A ordem 1 → 2 → 6 não deixava concluir a etapa 1 | dependência declarada na própria etapa 1 | Ordem nova: 6 → 1, 2 e 4 juntas → 3 → 5 |
| 11 | média | A tag `v0.0.0-test` já é recusada pelo teste de versão; não prova o portão | `release.yml`, passo "The tag names the crate's version"; `Cargo.toml` 0.1.0 | Aceitação da etapa 5 no próximo release real, olhando o estado `waiting` e os jobs posteriores |
| 12 | média | `println!` em qualquer ponto de `src/` não falha: `main.rs` é outra raiz | `src/lib.rs` linhas 5–9 | Mutação definida numa função da biblioteca, com o lint esperado |
| 13 | baixa | "Nada compila antes da aprovação" é falso: `gates` compila antes | `release.yml`: `draft` tem `needs: gates` | Texto corrigido: a aprovação antecede o rascunho e os artefatos, não as verificações |
| 14 | baixa | O handoff não "para no D-169": a seção da T5.3 já tem D-185 a D-187 | `handoff.md` linhas 160 e 326 | Etapa 1 passa a pedir a reconciliação do handoff; a ressalva no relatório foi corrigida |

Depois da revisão, o mantenedor decidiu não isolar as sessões de agente e aceitar o risco
(D-192, depois de D-190 e D-191). A etapa 4 foi reescrita como registro desse risco, e os achados
1, 2 e 4 da tabela ficam respondidos por ele: o plano não finge que um token ou um environment
resolvem. Os itens de credenciais e de humano no caminho continuam reprovados.
