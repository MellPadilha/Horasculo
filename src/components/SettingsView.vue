<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { MockUser, MockUserInput } from '../services/mockApi'

type Project = { id: string; name: string; color: string; tint: string }
type HoursEntry = { userId: string; projectId: string; hours: number }
type ExportEntry = { userId: string; title: string; start: string; end: string }

const props = withDefaults(defineProps<{
  projects: Project[]
  users: MockUser[]
  hoursEntries: HoursEntry[]
  exportEntries?: ExportEntry[]
  currentUserId: string
  loadingUsers: boolean
  errorMessage: string
  weeklyGoal: number
}>(), {
  exportEntries: () => [],
})

const emit = defineEmits<{
  addProject: [project: Project]
  deleteProject: [projectId: string]
  updateWeeklyGoal: [goal: number]
  saveUser: [user: MockUserInput]
  deleteUser: [userId: string]
}>()

const projectName = ref('')
const weeklyGoalInput = ref(props.weeklyGoal)
const projectPalette = [
  { name: 'Coral', color: '#c95f45', tint: '#f7c8bb' },
  { name: 'Verde', color: '#267d69', tint: '#bfe4d8' },
  { name: 'Azul', color: '#4f70b5', tint: '#c9d8f5' },
  { name: 'Amarelo', color: '#a87825', tint: '#f2dead' },
  { name: 'Lilás', color: '#7559a6', tint: '#ddd0f2' },
  { name: 'Rosa', color: '#a95073', tint: '#f2c9d9' },
  { name: 'Ciano', color: '#287b87', tint: '#bfe3e7' },
  { name: 'Pêssego', color: '#b9683c', tint: '#f5d0b9' },
]
const projectColor = ref(projectPalette[2].color)
const selectedUser = ref('all')
const projectDialog = ref(false)
const userDialog = ref(false)
const exportDialog = ref(false)
const projectError = ref('')
const weeklyGoalError = ref('')
const exportError = ref('')
const userError = ref('')
const userForm = reactive<MockUserInput>({ name: '', username: '', password: '' })
const today = new Date()
const formatDate = (date: Date) => new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: '2-digit', year: 'numeric' }).format(date)
const firstDay = new Date(today.getFullYear(), today.getMonth(), 1)
const exportRange = reactive({ start: formatDate(firstDay), end: formatDate(today) })

const dashboardRows = computed(() => props.projects.map((project) => ({
  ...project,
  hours: props.hoursEntries
    .filter((entry) => entry.projectId === project.id && (selectedUser.value === 'all' || entry.userId === selectedUser.value))
    .reduce((total, entry) => total + entry.hours, 0),
})))

const dashboardTotal = computed(() => dashboardRows.value.reduce((total, project) => total + project.hours, 0))
const userFilterItems = computed(() => [{ id: 'all', name: 'Todos os usuários' }, ...props.users])

function addProject() {
  const name = projectName.value.trim()
  projectError.value = ''
  if (!name) {
    projectError.value = 'Informe o nome do projeto.'
    return
  }
  const selectedColor = projectPalette.find((option) => option.color === projectColor.value) ?? projectPalette[0]
  emit('addProject', {
    id: `project-${Date.now()}`,
    name,
    color: selectedColor.color,
    tint: selectedColor.tint,
  })
  projectName.value = ''
  projectDialog.value = false
}

function openNewProject() {
  projectName.value = ''
  projectColor.value = projectPalette[2].color
  projectError.value = ''
  projectDialog.value = true
}

function saveWeeklyGoal() {
  const goal = Number(weeklyGoalInput.value)
  weeklyGoalError.value = ''
  if (!Number.isFinite(goal) || goal <= 0 || goal > 168) {
    weeklyGoalError.value = 'Informe uma meta entre 0,25 e 168 horas.'
    return
  }
  emit('updateWeeklyGoal', goal)
}

function openNewUser() {
  Object.assign(userForm, { id: undefined, name: '', username: '', password: '' })
  userError.value = ''
  userDialog.value = true
}

function openEditUser(user: MockUser) {
  Object.assign(userForm, { id: user.id, name: user.name, username: user.username, password: '' })
  userError.value = ''
  userDialog.value = true
}

function submitUser() {
  userError.value = ''
  if (!userForm.name.trim() || !userForm.username.trim()) {
    userError.value = 'Informe nome e usuário.'
    return
  }
  if (!userForm.id && !userForm.password) {
    userError.value = 'Informe uma senha para o novo usuário.'
    return
  }
  emit('saveUser', { ...userForm })
  userDialog.value = false
}

function formatHours(hours: number) {
  return hours.toLocaleString('pt-BR', { maximumFractionDigits: 2 })
}

function escapeXml(value: string) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;')
}

function uniqueSheetName(name: string, usedNames: Set<string>) {
  const base = name.replace(/[\\/?*\[\]:]/g, ' ').trim().slice(0, 31) || 'Usuário'
  let candidate = base
  let suffix = 2
  while (usedNames.has(candidate)) {
    const ending = ` (${suffix++})`
    candidate = `${base.slice(0, 31 - ending.length)}${ending}`
  }
  usedNames.add(candidate)
  return candidate
}

function exportSpreadsheet() {
  exportError.value = ''
  if (!exportRange.start || !exportRange.end) {
    exportError.value = 'Selecione a data inicial e a data final.'
    return
  }

  const parseDate = (value: string, endOfDay = false) => {
    const match = value.match(/^(\d{2})\/(\d{2})\/(\d{4})$/)
    if (!match) return new Date(Number.NaN)
    const [, day, month, year] = match.map(Number)
    const date = new Date(year, month - 1, day, endOfDay ? 23 : 0, endOfDay ? 59 : 0, endOfDay ? 59 : 0, endOfDay ? 999 : 0)
    return date.getFullYear() === year && date.getMonth() === month - 1 && date.getDate() === day
      ? date
      : new Date(Number.NaN)
  }
  const rangeStart = parseDate(exportRange.start)
  const rangeEnd = parseDate(exportRange.end, true)
  if (Number.isNaN(rangeStart.getTime()) || Number.isNaN(rangeEnd.getTime())) {
    exportError.value = 'Informe as datas no formato DD/MM/AAAA.'
    return
  }
  if (rangeEnd < rangeStart) {
    exportError.value = 'A data final deve ser igual ou posterior à data inicial.'
    return
  }

  const header = ['Nome do evento', 'Início', 'Fim', 'Data']
    .map((label) => `<Cell ss:StyleID="Header"><Data ss:Type="String">${label}</Data></Cell>`)
    .join('')
  const usedNames = new Set<string>()
  const worksheets = props.users.map((user) => {
    const rows = (props.exportEntries ?? [])
      .filter((entry) => entry.userId === user.id)
      .filter((entry) => {
        const start = new Date(entry.start)
        return start >= rangeStart && start <= rangeEnd
      })
      .sort((left, right) => new Date(left.start).getTime() - new Date(right.start).getTime())
      .map((entry) => {
        const start = new Date(entry.start)
        const end = new Date(entry.end)
        const values = [
          entry.title,
          start.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' }),
          end.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' }),
          formatDate(start),
        ]
        return `<Row>${values.map((value) => `<Cell><Data ss:Type="String">${escapeXml(value)}</Data></Cell>`).join('')}</Row>`
      })
      .join('')
    const sheetName = uniqueSheetName(user.name, usedNames)
    return `<Worksheet ss:Name="${escapeXml(sheetName)}"><Table><Column ss:Width="220"/><Column ss:Width="75"/><Column ss:Width="75"/><Column ss:Width="85"/><Row>${header}</Row>${rows}</Table></Worksheet>`
  }).join('')

  const workbook = `<?xml version="1.0" encoding="UTF-8"?><?mso-application progid="Excel.Sheet"?><Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"><Styles><Style ss:ID="Header"><Font ss:Bold="1"/><Interior ss:Color="#EAF4EF" ss:Pattern="Solid"/></Style></Styles>${worksheets}</Workbook>`
  const blob = new Blob([workbook], { type: 'application/vnd.ms-excel;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `horasculo-${exportRange.start.replaceAll('/', '-')}-a-${exportRange.end.replaceAll('/', '-')}.xls`
  document.body.appendChild(link)
  link.click()
  link.remove()
  URL.revokeObjectURL(url)
  exportDialog.value = false
}
</script>

<template>
  <main class="settings-content">
    <header class="settings-header">
      <div class="page-heading">
        <span class="eyebrow">ADMINISTRAÇÃO</span>
        <h1>Configurações</h1>
      </div>
      <v-btn class="export-button" variant="outlined" prepend-icon="mdi-file-export-outline" @click="exportError = ''; exportDialog = true">Exportar dados</v-btn>
    </header>
    <v-alert v-if="errorMessage" class="settings-alert" type="error" variant="tonal" density="compact">{{ errorMessage }}</v-alert>

    <section class="settings-dashboard">
      <div class="dashboard-heading">
        <div>
          <span class="eyebrow">VISÃO GERAL</span>
          <h2>Horas por projeto</h2>
        </div>
        <v-select v-model="selectedUser" class="dashboard-filter" :items="userFilterItems" item-title="name" item-value="id" label="Usuário" variant="outlined" density="compact" hide-details />
      </div>
      <div class="dashboard-total"><strong>{{ formatHours(dashboardTotal) }}h</strong><span>Total registrado</span></div>
      <div class="project-summary-grid">
        <article v-for="project in dashboardRows" :key="project.id" class="project-summary-card">
          <span class="project-dot" :style="{ backgroundColor: project.color }" />
          <div><strong>{{ formatHours(project.hours) }}h</strong><span>{{ project.name }}</span></div>
        </article>
      </div>
    </section>

    <section class="weekly-goal-settings">
      <div>
        <span class="eyebrow">JORNADA</span>
        <h2>Meta semanal</h2>
        <p>Defina quantas horas você pretende registrar por semana.</p>
      </div>
      <form class="weekly-goal-form" @submit.prevent="saveWeeklyGoal">
        <v-text-field v-model.number="weeklyGoalInput" label="Horas por semana" type="number" min="0.25" max="168" step="0.25" suffix="h" variant="outlined" density="compact" hide-details />
        <v-btn class="small-primary-button" type="submit">Salvar meta</v-btn>
        <span v-if="weeklyGoalError" class="weekly-goal-error">{{ weeklyGoalError }}</span>
      </form>
    </section>

    <div class="settings-grid">
      <section class="settings-card">
        <div class="settings-card-heading"><div><span class="eyebrow">CADASTRO</span><h2>Projetos</h2></div><div class="settings-heading-actions"><span>{{ projects.length }} projetos</span><v-btn class="small-primary-button" prepend-icon="mdi-plus" @click="openNewProject">Adicionar</v-btn></div></div>
        <v-table class="settings-table" density="compact">
          <thead><tr><th>Projeto</th><th class="table-actions">Ações</th></tr></thead>
          <tbody>
            <tr v-for="project in projects" :key="project.id">
              <td><span class="table-project"><span class="project-dot" :style="{ backgroundColor: project.color }" />{{ project.name }}</span></td>
              <td class="table-actions"><v-btn icon="mdi-trash-can-outline" variant="text" size="small" color="error" :aria-label="`Excluir ${project.name}`" @click="emit('deleteProject', project.id)" /></td>
            </tr>
          </tbody>
        </v-table>
      </section>

      <section class="settings-card">
        <div class="settings-card-heading"><div><span class="eyebrow">ACESSOS</span><h2>Usuários</h2></div><v-btn class="small-primary-button" prepend-icon="mdi-plus" @click="openNewUser">Adicionar</v-btn></div>
        <v-progress-linear v-if="loadingUsers" indeterminate color="#d96446" />
        <v-table class="settings-table" density="compact">
          <thead><tr><th>Nome</th><th>Usuário</th><th class="table-actions">Ações</th></tr></thead>
          <tbody>
            <tr v-for="user in users" :key="user.id">
              <td>{{ user.name }} <span v-if="user.id === currentUserId" class="you-badge">Você</span></td>
              <td>@{{ user.username }}</td>
              <td class="table-actions">
                <v-btn icon="mdi-pencil-outline" variant="text" size="small" :aria-label="`Editar ${user.name}`" @click="openEditUser(user)" />
                <v-btn icon="mdi-trash-can-outline" variant="text" size="small" color="error" :disabled="user.id === currentUserId" :aria-label="`Excluir ${user.name}`" @click="emit('deleteUser', user.id)" />
              </td>
            </tr>
          </tbody>
        </v-table>
      </section>
    </div>

    <v-dialog v-model="projectDialog" max-width="420">
      <v-card class="event-dialog project-dialog">
        <form @submit.prevent="addProject">
          <div class="dialog-heading"><div><div class="eyebrow">PROJETO</div><h2>Novo projeto</h2></div><v-btn icon="mdi-close" variant="text" @click="projectDialog = false" /></div>
          <v-alert v-if="projectError" class="mb-4" type="error" variant="tonal" density="compact">{{ projectError }}</v-alert>
          <v-text-field v-model="projectName" label="Nome do projeto" variant="outlined" density="comfortable" autofocus />
          <fieldset class="project-color-fieldset">
            <legend>Cor do projeto</legend>
            <div class="color-palette" role="radiogroup" aria-label="Cor do projeto">
              <button
                v-for="option in projectPalette"
                :key="option.color"
                class="color-swatch"
                :class="{ 'color-swatch--active': projectColor === option.color }"
                :style="{ backgroundColor: option.tint, borderColor: option.color }"
                type="button"
                role="radio"
                :aria-checked="projectColor === option.color"
                :aria-label="option.name"
                :title="option.name"
                @click="projectColor = option.color"
              />
            </div>
          </fieldset>
          <div class="modal-footer"><v-btn variant="text" @click="projectDialog = false">Cancelar</v-btn><v-btn class="small-primary-button" type="submit">Adicionar projeto</v-btn></div>
        </form>
      </v-card>
    </v-dialog>

    <v-dialog v-model="userDialog" max-width="480">
      <v-card class="event-dialog">
        <div class="dialog-heading"><div><div class="eyebrow">USUÁRIO</div><h2>{{ userForm.id ? 'Editar usuário' : 'Adicionar usuário' }}</h2></div><v-btn icon="mdi-close" variant="text" @click="userDialog = false" /></div>
        <v-alert v-if="userError" class="mb-4" type="error" variant="tonal" density="compact">{{ userError }}</v-alert>
        <v-text-field v-model="userForm.name" label="Nome" variant="outlined" density="comfortable" />
        <v-text-field v-model="userForm.username" label="Usuário" variant="outlined" density="comfortable" />
        <v-text-field v-model="userForm.password" :label="userForm.id ? 'Nova senha (opcional)' : 'Senha'" type="password" variant="outlined" density="comfortable" />
        <div class="modal-footer"><v-btn variant="text" @click="userDialog = false">Cancelar</v-btn><v-btn class="small-primary-button" @click="submitUser">Salvar</v-btn></div>
      </v-card>
    </v-dialog>

    <v-dialog v-model="exportDialog" max-width="500">
      <v-card class="event-dialog">
        <div class="dialog-heading"><div><div class="eyebrow">PLANILHA</div><h2>Exportar dados</h2></div><v-btn icon="mdi-close" variant="text" @click="exportDialog = false" /></div>
        <p class="export-copy">Selecione o período que será usado na exportação de todos os usuários.</p>
        <v-alert v-if="exportError" class="mb-4" type="error" variant="tonal" density="compact">{{ exportError }}</v-alert>
        <div class="time-fields">
          <v-text-field v-model="exportRange.start" label="Data inicial" placeholder="DD/MM/AAAA" variant="outlined" density="comfortable" />
          <v-text-field v-model="exportRange.end" label="Data final" placeholder="DD/MM/AAAA" variant="outlined" density="comfortable" />
        </div>
        <div class="modal-footer"><v-btn variant="text" @click="exportDialog = false">Cancelar</v-btn><v-btn class="small-primary-button" prepend-icon="mdi-download" @click="exportSpreadsheet">Exportar planilha</v-btn></div>
      </v-card>
    </v-dialog>
  </main>
</template>
