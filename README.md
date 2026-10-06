# Horasculo

![Ícone do Horasculo](public/brand/horasculo-icon.png)

O Horasculo é uma interface de controle de horas de trabalho. O sistema organiza registros em um calendário, separa as horas por projeto e apresenta totais diários e semanais para facilitar o acompanhamento da jornada.

## Estado do projeto

Este projeto contém **somente o front-end**. Não há API, banco de dados ou servidor de autenticação.

Os usuários, projetos, registros, sessão e metas semanais são simulados e armazenados no `localStorage` do navegador. Portanto, os dados:

- ficam disponíveis apenas no navegador e dispositivo em que foram criados;
- podem ser apagados ao limpar os dados do navegador;
- não são sincronizados entre dispositivos;
- não devem ser considerados seguros para uso em produção.

## Acesso de demonstração

- Usuário: `demo`
- Senha: `12345`

## Funcionalidades implementadas

### Calendário e registros

- Visualização semanal e diária.
- Navegação entre semanas e retorno rápido para o dia atual.
- Grade de 24 horas, iniciando às `06:00` e terminando às `06:00` do dia seguinte.
- Criação de registros selecionando um período diretamente no calendário.
- Criação, edição e exclusão de registros de horas.
- Movimentação e redimensionamento dos registros por arrastar e soltar.
- Associação de cada registro a um projeto.
- Filtro do calendário por projeto.
- Cálculo automático da duração ao alterar os horários de início ou término.
- Cálculo automático do horário de término ao alterar o total de horas trabalhadas.
- Suporte a registros de 24 horas e a registros que atravessam a madrugada.
- Soma das horas no cabeçalho de cada dia, respeitando o projeto filtrado.
- Total de horas e quantidade de registros no período visível.

### Projetos

- Cadastro de projetos em um modal compacto.
- Seleção por uma paleta de cores pastel opacas.
- Listagem e exclusão de projetos.
- Identificação visual dos projetos no calendário, menu lateral e indicadores.

### Meta semanal

- Configuração da meta semanal de horas para cada usuário.
- Persistência individual da meta no navegador.
- Indicador de progresso semanal no menu lateral.
- Exibição das horas restantes para alcançar a meta.

### Usuários

- Login e logout simulados.
- Cadastro, edição e exclusão de usuários.
- Proteção contra a exclusão do usuário atualmente conectado.
- Registros de calendário separados por usuário.

### Relatórios e configurações

- Visão geral das horas registradas por projeto.
- Filtro dos totais por usuário.
- Exportação dos registros para uma planilha `.xls`.
- Seleção do período usado na exportação.
- Uma aba da planilha para cada usuário.

### Interface

- Layout responsivo para desktop e telas menores.
- Identidade visual própria no login e no menu lateral.
- Favicon e ícone para atalhos em dispositivos móveis.

## Ícone do sistema

O emblema de olho e relógio representa a proposta do Horasculo: visualizar, registrar e acompanhar o tempo de trabalho.

Os arquivos usados pela aplicação estão em:

- `public/brand/horasculo-icon.png`: marca exibida na interface e em atalhos;
- `public/brand/horasculo-favicon.png`: ícone exibido na aba do navegador.

## Tecnologias

- Vue 3
- TypeScript
- Vite
- Vuetify
- FullCalendar
- Material Design Icons

## Como executar

Requisitos: Node.js e npm instalados.

```bash
npm install
npm run dev
```

O terminal exibirá o endereço local da aplicação.

## Compilação de produção

```bash
npm run build
```

Para visualizar localmente o resultado compilado:

```bash
npm run preview
```
