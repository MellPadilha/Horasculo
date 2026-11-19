<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { User } from '../services/api'

type Project = { id: string; name: string; color: string; tint: string }
type HoursEntry = { userId: string; projectId: string; hours: number }

const props = defineProps<{
  projects: Project[]
  users: User[]
  hoursEntries: HoursEntry[]
  summaryFrom: Date
  summaryTo: Date
  loadingSummary: boolean
  currentUserId: string
  loadingUsers: boolean
  errorMessage: string
  weeklyGoal: number
  isAdmin: boolean
}>()

const emit = defineEmits<{
  addProject: [project: Project]
  deleteProject: [projectId: string]
  updateWeeklyGoal: [goal: number]
  revokeUserSessions: [userId: string]
  exportReport: [from: Date, to: Date]
  updateSummaryRange: [from: Date, to: Date]
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
const exportDialog = ref(false)
const projectError = ref('')
const weeklyGoalError = ref('')
const exportError = ref('')
const dashboardError = ref('')
const today = new Date()
const firstDay = new Date(today.getFullYear(), today.getMonth(), 1)
const exportRange = reactive<{ start: Date | null; end: Date | null }>({ start: firstDay, end: today })
const dashboardRange = reactive<{ start: Date | null; end: Date | null }>({ start: props.summaryFrom, end: new Date(props.summaryTo.getTime() - 1) })

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

function formatHours(hours: number) {
  return hours.toLocaleString('pt-BR', { maximumFractionDigits: 2 })
}

function validDate(value: Date | null): value is Date {
  return value instanceof Date && !Number.isNaN(value.getTime())
}

function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}

function nextDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1)
}

function updateDashboardRange() {
  dashboardError.value = ''
  if (!validDate(dashboardRange.start) || !validDate(dashboardRange.end)) {
    dashboardError.value = 'Selecione a data inicial e a data final.'
    return
  }
  const from = startOfDay(dashboardRange.start)
  const to = nextDay(dashboardRange.end)
  if (to <= from) {
    dashboardError.value = 'A data final deve ser igual ou posterior à inicial.'
    return
  }
  if (to.getTime() - from.getTime() > 366 * 24 * 60 * 60 * 1000) {
    dashboardError.value = 'Selecione um período de até 366 dias.'
    return
  }
  emit('updateSummaryRange', from, to)
}

function exportSpreadsheet() {
  exportError.value = ''
  if (!validDate(exportRange.start) || !validDate(exportRange.end)) {
    exportError.value = 'Selecione a data inicial e a data final.'
    return
  }

  const rangeStart = startOfDay(exportRange.start)
  const rangeEnd = nextDay(exportRange.end)
  if (rangeEnd <= rangeStart) {
    exportError.value = 'A data final deve ser igual ou posterior à data inicial.'
    return
  }

  emit('exportReport', rangeStart, rangeEnd)
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
      <v-btn v-if="isAdmin" class="export-button" variant="outlined" prepend-icon="mdi-file-export-outline" @click="exportError = ''; exportDialog = true">Exportar dados</v-btn>
    </header>
    <v-alert v-if="errorMessage" class="settings-alert" type="error" variant="tonal" density="compact">{{ errorMessage }}</v-alert>

    <section v-if="isAdmin" class="settings-dashboard">
      <div class="dashboard-heading">
        <div>
          <span class="eyebrow">VISÃO GERAL</span>
          <h2>Horas por projeto</h2>
        </div>
      </div>
      <form class="dashboard-filters" @submit.prevent="updateDashboardRange">
        <v-select v-model="selectedUser" class="dashboard-filter" :items="userFilterItems" item-title="name" item-value="id" label="Usuário" variant="outlined" density="compact" hide-details />
        <v-date-input v-model="dashboardRange.start" label="De" input-format="dd/mm/yyyy" variant="outlined" density="compact" hide-details />
        <v-date-input v-model="dashboardRange.end" label="Até" input-format="dd/mm/yyyy" variant="outlined" density="compact" hide-details />
        <v-btn class="small-primary-button" type="submit" :loading="loadingSummary">Aplicar período</v-btn>
      </form>
      <v-alert v-if="dashboardError" class="dashboard-period-error" type="error" variant="tonal" density="compact">{{ dashboardError }}</v-alert>
      <v-progress-linear v-if="loadingSummary" class="dashboard-progress" indeterminate color="#d96446" />
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

    <div v-if="isAdmin" class="settings-grid">
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
        <div class="settings-card-heading"><div><span class="eyebrow">ACESSOS</span><h2>Usuários</h2></div><span>Gerencie acessos no Microsoft Entra ID</span></div>
        <v-progress-linear v-if="loadingUsers" indeterminate color="#d96446" />
        <v-table class="settings-table" density="compact">
          <thead><tr><th>Nome</th><th>Usuário</th><th class="table-actions">Ações</th></tr></thead>
          <tbody>
            <tr v-for="user in users" :key="user.id">
              <td>{{ user.name }} <span v-if="user.id === currentUserId" class="you-badge">Você</span></td>
              <td>{{ user.username }}</td>
              <td class="table-actions">
                <v-btn icon="mdi-logout" variant="text" size="small" :aria-label="`Revogar sessões de ${user.name}`" @click="emit('revokeUserSessions', user.id)" />
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

    <v-dialog v-model="exportDialog" max-width="500">
      <v-card class="event-dialog">
        <div class="dialog-heading"><div><div class="eyebrow">PLANILHA</div><h2>Exportar dados</h2></div><v-btn icon="mdi-close" variant="text" @click="exportDialog = false" /></div>
        <p class="export-copy">Selecione o período que será usado na exportação de todos os usuários.</p>
        <v-alert v-if="exportError" class="mb-4" type="error" variant="tonal" density="compact">{{ exportError }}</v-alert>
        <div class="time-fields">
          <v-date-input v-model="exportRange.start" label="Data inicial" input-format="dd/mm/yyyy" variant="outlined" density="comfortable" />
          <v-date-input v-model="exportRange.end" label="Data final" input-format="dd/mm/yyyy" variant="outlined" density="comfortable" />
        </div>
        <div class="modal-footer"><v-btn variant="text" @click="exportDialog = false">Cancelar</v-btn><v-btn class="small-primary-button" prepend-icon="mdi-download" @click="exportSpreadsheet">Exportar planilha</v-btn></div>
      </v-card>
    </v-dialog>
  </main>
</template>
