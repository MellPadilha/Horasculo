# Horasculo

![Ícone do Horasculo](apps/web/public/brand/horasculo-icon.png)

O Horasculo é uma interface de controle de horas de trabalho. O sistema organiza registros em um calendário, separa as horas por projeto e apresenta totais diários e semanais para facilitar o acompanhamento da jornada.

## Estrutura do projeto

O Horasculo é organizado como um monorepo:

```text
apps/
├── api/    # API Rust com Axum
└── web/    # front-end Vue 3
```

O front consome a API Rust, que persiste usuários, projetos, metas e registros no PostgreSQL. A autenticação usa uma sessão em cookie `HttpOnly`.

## Primeiro acesso

No primeiro boot, a API cria o administrador definido por `BOOTSTRAP_ADMIN_USERNAME` e `BOOTSTRAP_ADMIN_PASSWORD`. Use uma senha exclusiva com pelo menos 12 caracteres.

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
- Persistência individual da meta no PostgreSQL.
- Indicador de progresso semanal no menu lateral.
- Exibição das horas restantes para alcançar a meta.

### Usuários

- Login e logout com sessão segura em cookie `HttpOnly`.
- Cadastro, edição e exclusão de usuários.
- Proteção contra a exclusão do usuário atualmente conectado.
- Registros de calendário separados por usuário.

### Relatórios e configurações

- Visão geral das horas registradas por projeto.
- Filtro dos totais por usuário.
- Exportação segura dos registros para uma planilha `.xlsx`, gerada pela API.
- Seleção do período usado na exportação.
- Uma aba da planilha para cada usuário.

### Interface

- Layout responsivo para desktop e telas menores.
- Identidade visual própria no login e no menu lateral.
- Favicon e ícone para atalhos em dispositivos móveis.

## Ícone do sistema

O emblema de olho e relógio representa a proposta do Horasculo: visualizar, registrar e acompanhar o tempo de trabalho.

Os arquivos usados pela aplicação estão em:

- `apps/web/public/brand/horasculo-icon.png`: marca exibida na interface e em atalhos;
- `apps/web/public/brand/horasculo-favicon.png`: ícone exibido na aba do navegador.

## Tecnologias

- Vue 3
- TypeScript
- Vite
- Vuetify
- FullCalendar
- Material Design Icons
- Rust
- Axum
- Tokio

## Como executar

### Front-end

Requisitos: Node.js e npm instalados. Na raiz do repositório:

```bash
npm install
npm run dev:web
```

O terminal exibirá o endereço local da aplicação.

### API e banco

Requisitos: Rust, Cargo e Docker instalados. Na raiz do repositório:

```bash
docker compose up -d postgres
cp apps/api/.env.example apps/api/.env
set -a; source apps/api/.env; set +a
cargo run -p horasculo-api
```

A API será iniciada em `http://127.0.0.1:3000`. O endpoint inicial é:

```text
GET /api/health
GET /api/ready
```

As variáveis disponíveis estão documentadas em `apps/api/.env.example`. Elas podem ser definidas no ambiente antes de iniciar a API.

## Compilação de produção

```bash
npm run build:web
```

Para visualizar localmente o resultado compilado:

```bash
npm run preview:web
```

Para verificar o backend:

```bash
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Produção com HTTPS

O arquivo `compose.prod.yaml` inicia PostgreSQL, API, front-end e Caddy. O Caddy obtém e renova automaticamente o certificado TLS do domínio informado.

Crie um arquivo `.env` no servidor, fora do controle de versão:

```dotenv
DOMAIN=horas.exemplo.com
POSTGRES_PASSWORD=uma-senha-longa-e-exclusiva
BOOTSTRAP_ADMIN_USERNAME=admin
BOOTSTRAP_ADMIN_PASSWORD=outra-senha-longa-com-pelo-menos-12-caracteres
BOOTSTRAP_ADMIN_NAME=Administrador
```

Depois execute:

```bash
docker compose -f compose.prod.yaml up -d --build
```

O DNS do domínio deve apontar para o servidor e as portas 80 e 443 devem estar liberadas. O PostgreSQL não é publicado na internet.

### Backup e restauração

Faça diariamente um dump consistente e armazene-o criptografado fora do servidor:

```bash
docker compose -f compose.prod.yaml exec -T postgres pg_dump -U horasculo -d horasculo -Fc > horasculo.dump
```

Mantenha pelo menos 7 backups diários e 4 semanais. Teste regularmente a restauração em um banco separado:

```bash
docker compose -f compose.prod.yaml exec -T postgres createdb -U horasculo horasculo_restore
docker compose -f compose.prod.yaml exec -T postgres pg_restore -U horasculo -d horasculo_restore --clean --if-exists < horasculo.dump
```

Antes de atualizar, gere um backup, construa as novas imagens e acompanhe os logs da API enquanto as migrações são aplicadas.
