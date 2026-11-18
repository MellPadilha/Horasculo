# Implantação na AWS

Esta configuração usa uma EC2, PostgreSQL em volume local criptografado, Caddy para HTTPS e backups em S3. A disponibilidade fica limitada a uma VM; uma falha da instância exige restauração. Não aplique o Terraform antes de definir domínio, conta AWS, tenant Entra e responsável pelos alertas.

## Microsoft Entra ID

1. Registre um aplicativo **single-tenant** com URI de retorno Web `https://DOMINIO/api/auth/callback`.
2. Crie os papéis de aplicativo `Horasculo.User` e `Horasculo.Admin` para membros do tipo User.
3. Na aplicação empresarial, ative **Assignment required** e conceda consentimento administrativo às permissões OIDC solicitadas.
4. Atribua **individualmente** cada conta autorizada ao papel `Horasculo.User`. Atribua o grupo administrativo ao papel `Horasculo.Admin`. A API exige o papel User mesmo para quem recebe Admin.
5. Aplique a política corporativa de acesso condicional com MFA. Atribuição por grupo exige Entra ID P1 ou P2.

## Terraform

Instale Terraform 1.10+ e configure credenciais AWS com permissão de provisionamento. Execute `infra/bootstrap` primeiro:

```bash
cd infra/bootstrap
terraform init
terraform apply -var='aws_region=sa-east-1'
```

Guarde o estado local inicial em local protegido. Em seguida inicialize `infra/prod` com o bucket retornado:

```bash
cd ../prod
terraform init -backend-config='bucket=BUCKET_DE_STATE' -backend-config='key=prod/terraform.tfstate' -backend-config='region=sa-east-1'
terraform plan -var='aws_region=sa-east-1' -var='domain=horas.minhaempresa.com' -var='route53_zone_id=ZONE_ID' -var='alert_email=seguranca@minhaempresa.com'
terraform apply -var='aws_region=sa-east-1' -var='domain=horas.minhaempresa.com' -var='route53_zone_id=ZONE_ID' -var='alert_email=seguranca@minhaempresa.com'
```

Confirme a assinatura do tópico SNS recebida no e-mail de alertas. O Terraform cria **apenas os nomes** dos segredos no Secrets Manager; não grave valores em variáveis, planos ou estado Terraform.

## Segredos e primeira implantação

Preencha no Secrets Manager os cinco segredos mostrados pelo output `secret_names`:

| Segredo | Valor |
|---|---|
| `db_admin_password` | Senha forte do papel PostgreSQL `horasculo_migrator` |
| `db_app_password` | Senha forte do papel `horasculo_app` |
| `migration_database_url` | `postgres://horasculo_migrator:SENHA@postgres:5432/horasculo` |
| `database_url` | `postgres://horasculo_app:SENHA@postgres:5432/horasculo` |
| `entra_client_secret` | Segredo do registro do aplicativo Entra |

Codifique caracteres especiais das senhas nas URLs PostgreSQL. Não compartilhe o segredo de migração com o serviço da API.

Use Session Manager para acessar a VM, copie o repositório para `/opt/horasculo` e crie `/etc/horasculo/production.env` a partir de `ops/production.env.example`. Preencha os outputs Terraform (incluindo `alert_topic`), tenant e client ID do Entra; deixe o arquivo acessível apenas a root (`chmod 600`). Execute `sudo /opt/horasculo/ops/deploy.sh`. O serviço `horasculo-deploy.service` refaz a carga dos segredos em `/run` após reinicialização.

O script de inicialização PostgreSQL cria o papel restrito somente quando o volume de produção está vazio. Se já houver um volume existente, prepare a migração do papel antes de atualizar.

## Operação

- Execute `sudo systemctl status horasculo-deploy.service horasculo-backup.timer horasculo-restore-check.timer` depois da implantação.
- Teste uma conta atribuída, uma não atribuída, um membro do grupo Admin sem o papel User e um usuário comum. Teste também MFA, revogação de sessões, HTTP para HTTPS e os cabeçalhos de segurança.
- Antes de atualizar, execute `sudo /opt/horasculo/ops/backup.sh` e verifique o objeto no S3. Depois rode `sudo /opt/horasculo/ops/deploy.sh`.
- O backup diário roda às 03:00 UTC e o teste de restauração no primeiro dia de cada mês. O S3 mantém 7 dias de cópias diárias e 28 dias de cópias semanais; versões antigas expiram em até mais 30 dias. Logs ficam 90 dias no CloudWatch.
- Para desligamento imediato, remova a atribuição User no Entra e revogue as sessões no painel do Horasculo. Remover só o grupo Admin reduz o papel no próximo login; revogue sessões para aplicação imediata.
- Não há exclusão automática de registros de horas enquanto a política corporativa de retenção não estiver definida.
- A CI bloqueia vulnerabilidades altas ou críticas nas imagens da API e do site. Ela também publica a varredura das imagens PostgreSQL e Caddy para revisão: os avisos atuais recaem sobre bibliotecas Go embutidas em `gosu` e no parser HTML do Caddy, fora das funções usadas aqui. Revise essa avaliação ao atualizar as imagens.
- A auditoria Rust ignora dois avisos documentados na CI: `rsa` é usado apenas para verificar assinaturas públicas, e SQLite é um recurso opcional do SQLx que não entra no binário PostgreSQL. Remova as exceções quando as dependências transitivas forem atualizadas.
- A auditoria também informa que `chrono` está sem manutenção, sem versão corrigida disponível no momento. Acompanhe a substituição nas atualizações do projeto.
