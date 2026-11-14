# PRD: <nome> — <o que muda, numa linha>

**Status:** rascunho · **Versão:** 0.1 · **Data:** AAAA-MM-DD
**Decisão que o abre:** D-NNN, se houver.

<!--
Molde de spec deste repositório: a forma que `spec/ripwire-broker-mcp.md` e `docs/jev-mem-prd.md`
já usam. Em português, como o resto de `spec/`; nomes de código, flags e comandos ficam como são.
Apague estes comentários ao preencher. Uma seção que não se aplica fica, com uma linha dizendo
por quê: a ausência não diz se foi pensada.
-->

## Contexto e problema

<!-- O que existe hoje, o que dá errado, para quem, e a evidência (medição, issue, D-NNN). Sem
solução aqui. -->

## Objetivos e não objetivos

**Objetivos**

1. <resultado observável>

**Não objetivos**

1. <o que fica de fora de propósito, e por quê>

## Critérios de aceitação

<!-- Numerados e verificáveis, um por comportamento, no formato do PRD (CA-01 a CA-10). Cada
critério vira pelo menos um teste em `tests/`, escrito vermelho antes do código. -->

### CA-01 — <nome curto>

**Dado** <o estado inicial>, **quando** <a ação>, **então** <o resultado observável>.

### CA-02 — <nome curto>

**Dado** <…>, **quando** <…>, **então** <…>.

## Falhas e comportamento obrigatório

<!-- Para cada falha previsível (rede, disco, processo morto no meio, entrada inválida, versão
antiga do outro lado): o que o sistema faz, e o que ele nunca faz. Falhar fechado é o padrão; uma
falha que passa em silêncio precisa de motivo escrito. -->

- **<falha>:** <comportamento obrigatório>.

## Como desfazer

<!-- Como voltar ao estado anterior: desligar a flag, reverter a versão, o que acontece com dados
já gravados num formato novo, e o que não volta sozinho. Se não há volta, dizer isso. -->

## Decisões relacionadas

<!-- As entradas de `spec/changelog.md` que esta spec cita, abre ou substitui. Toda decisão
tomada ao escrevê-la ou ao implementá-la vira uma entrada nova. -->

- D-NNN — <título>
