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

O front consome a API Rust, que persiste usuários, projetos, metas e registros no PostgreSQL. O login usa Microsoft Entra ID; a API mantém uma sessão em cookie `HttpOnly`.

## Primeiro acesso

No Entra ID, atribua diretamente cada pessoa autorizada ao papel de aplicativo `Horasculo.User`. Atribua o grupo administrativo ao papel `Horasculo.Admin`. A API exige `Horasculo.User` mesmo para membros desse grupo. Usuários são criados no banco no primeiro login autorizado.

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

- Login corporativo por OpenID Connect e logout local.
- Contas e papéis administrados no Entra ID; revogação de sessões pelo administrador.
- Registros de calendário separados por usuário.

### Relatórios e configurações

- Visão geral das horas registradas por projeto.
- Filtro dos totais por usuário.
- Filtro dos totais por intervalo de datas, incluindo o dia inicial e o final.
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

Requisitos: Node.js na versão indicada em `.nvmrc` e npm instalados. Na raiz do repositório:

```bash
nvm use
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
```

Antes de iniciar a API pela primeira vez, aplique as migrações com a credencial de migração:

```bash
cargo run -p horasculo-api -- migrate
```

Depois inicie normalmente:

```bash
cargo run -p horasculo-api
```

A API será iniciada em `http://127.0.0.1:3000`. O endpoint inicial é:

```text
GET /api/health
GET /api/ready
```

Preencha `ENTRA_TENANT_ID`, `ENTRA_CLIENT_ID` e `ENTRA_CLIENT_SECRET` no ambiente local. Registre `http://localhost:5173/api/auth/callback` como URI de retorno do tipo Web no Entra. O arquivo `apps/api/.env.example` contém as demais variáveis.

## Compilação de produção

```bash
npm run build:web
```

Para visualizar localmente o resultado compilado:

```bash
npm run preview:web
```

Esse comando recompila o frontend normalmente e abre o preview em `http://localhost:5173`. Inicie a API antes de acessar a aplicação; o login usa o Microsoft Entra ID e os dados vêm do PostgreSQL. O endereço coincide com `ALLOWED_ORIGIN` e com a URI de retorno indicados acima.

Para verificar o backend:

```bash
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Produção com HTTPS

O Terraform em [infra/README.md](infra/README.md) provisiona a VM, DNS, segredos, logs e backups na AWS. O `compose.prod.yaml` inicia PostgreSQL, migrações, API, front-end e Caddy; só o Caddy publica portas. Os valores dos segredos são carregados em arquivos temporários no servidor por `ops/deploy.sh`. O Caddy obtém e renova o certificado TLS.

### Backup e restauração

O timer `horasculo-backup.timer` envia um dump diário criptografado ao S3 e mantém cópias semanais. O timer `horasculo-restore-check.timer` testa mensalmente a restauração em um contêiner isolado. O CloudWatch alerta quando o backup diário falha ou deixa de acontecer. Registros de horas não são excluídos automaticamente; a empresa deve definir sua política de retenção.
