<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import FullCalendar from '@fullcalendar/vue3'
import timeGridPlugin from '@fullcalendar/timegrid'
import interactionPlugin from '@fullcalendar/interaction'
import ptBrLocale from '@fullcalendar/core/locales/pt-br'
import type { CalendarApi, CalendarOptions, DateSelectArg, DatesSetArg, EventInput } from '@fullcalendar/core'
import SettingsView from './components/SettingsView.vue'
import { api, type Project, type TimeEntry, type User, type UserInput } from './services/api'

const projects = ref<Project[]>([])

const hoursInput = (date: Date) => {
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${pad(date.getDate())}/${pad(date.getMonth() + 1)}/${date.getFullYear()} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

const parseDateTime = (value: string) => {
  const match = value.match(/^(\d{2})\/(\d{2})\/(\d{4})\s(\d{2}):(\d{2})$/)
  if (!match) return new Date(Number.NaN)
  const [, day, month, year, hour, minute] = match.map(Number)
  const date = new Date(year, month - 1, day, hour, minute)
  return date.getFullYear() === year && date.getMonth() === month - 1 && date.getDate() === day
    ? date
    : new Date(Number.NaN)
}

const weekStartFor = (date: Date) => {
  const start = new Date(date)
  start.setHours(0, 0, 0, 0)
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7))
  return start
}

const weekStart = ref(weekStartFor(new Date()))
const visibleRangeStart = ref(new Date(weekStart.value))
const visibleRangeEnd = ref(new Date(weekStart.value.getTime() + 7 * 24 * 60 * 60 * 1000))

const currentUser = ref<User | null>(null)
const weeklyGoal = ref(40)
const initializing = ref(true)
const currentPage = ref<'calendar' | 'settings'>('calendar')
const users = ref<User[]>([])
const adminEntries = ref<TimeEntry[]>([])
const loadingUsers = ref(false)
const settingsError = ref('')
const loginLoading = ref(false)
const loginError = ref('')
const loginForm = reactive({ username: '', password: '' })

const events = ref<EventInput[]>([])

const calendarRef = ref<{ getApi: () => CalendarApi } | null>(null)
const activeProject = ref('all')
const dialogOpen = ref(false)
const editingEventId = ref<string | null>(null)
const weekLabel = ref('')
const formError = ref('')
const currentView = ref<'timeGridWeek' | 'timeGridDay'>('timeGridWeek')
const form = reactive({
  title: '',
  projectId: projects.value[0]?.id ?? '',
  start: '',
  end: '',
  hours: 1,
})

watch(() => [form.start, form.end], ([startValue, endValue]) => {
  const start = parseDateTime(startValue)
  const end = parseDateTime(endValue)
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime()) || end <= start) return

  form.hours = Math.round((end.getTime() - start.getTime()) / 900_000) / 4
})

watch(() => form.hours, (hoursValue) => {
  const start = parseDateTime(form.start)
  const hours = Number(hoursValue)
  if (Number.isNaN(start.getTime()) || !Number.isFinite(hours) || hours <= 0) return

  form.end = hoursInput(new Date(start.getTime() + hours * 3_600_000))
})

const visibleEvents = computed(() => activeProject.value === 'all'
  ? events.value
  : events.value.filter((event) => event.extendedProps?.projectId === activeProject.value))

const dateKey = (date: Date) => {
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

const dailyHours = computed(() => visibleEvents.value.reduce<Record<string, number>>((totals, event) => {
  const start = new Date(event.start as string | Date)
  if (Number.isNaN(start.getTime())) return totals
  if (start.getHours() < 6) start.setDate(start.getDate() - 1)
  const key = dateKey(start)
  totals[key] = (totals[key] ?? 0) + Number(event.extendedProps?.hours ?? 0)
  return totals
}, {}))

const weekHours = computed(() => {
  const end = new Date(weekStart.value.getTime() + 7 * 24 * 60 * 60 * 1000)
  return events.value.reduce((total, event) => {
    const start = new Date(event.start as string | Date)
    return start >= weekStart.value && start < end ? total + Number(event.extendedProps?.hours ?? 0) : total
  }, 0)
})

const weekHoursLabel = computed(() => weekHours.value.toLocaleString('pt-BR', { maximumFractionDigits: 1 }))

function toCalendarEvent(entry: TimeEntry): EventInput {
  const project = projects.value.find((item) => item.id === entry.project_id)
  return { id: entry.id, title: entry.title, start: entry.starts_at, end: entry.ends_at,
    backgroundColor: project?.tint, borderColor: project?.color, textColor: '#252923',
    extendedProps: { projectId: entry.project_id, hours: entry.hours } }
}

async function loadEntries(from = visibleRangeStart.value, to = visibleRangeEnd.value) {
  if (!currentUser.value) return
  events.value = (await api.listEntries(from, to, { userId: currentUser.value.id })).map(toCalendarEvent)
}

async function loadWorkspace() {
  const [loadedProjects, goal] = await Promise.all([api.listProjects(), api.getWeeklyGoal()])
  projects.value = loadedProjects
  weeklyGoal.value = goal.hours
  form.projectId = projects.value[0]?.id ?? ''
  await loadEntries()
}

onMounted(async () => {
  try {
    currentUser.value = await api.me()
    await loadWorkspace()
  } catch {
    currentUser.value = null
  } finally {
    Object.keys(localStorage).filter((key) => key.startsWith('horas.')).forEach((key) => localStorage.removeItem(key))
    initializing.value = false
  }
})

async function submitLogin() {
  loginError.value = ''
  if (!loginForm.username.trim() || !loginForm.password) {
    loginError.value = 'Informe usuário e senha.'
    return
  }

  loginLoading.value = true
  try {
    const user = await api.login(loginForm.username, loginForm.password)
    currentUser.value = user
    await loadWorkspace()
    loginForm.password = ''
  } catch (error) {
    loginError.value = error instanceof Error ? error.message : 'Não foi possível entrar.'
  } finally {
    loginLoading.value = false
  }
}

async function logout() {
  try { await api.logout() } catch { /* Encerra o estado local mesmo se a rede falhar. */ }
  currentUser.value = null
  events.value = []
  loginForm.username = ''
  loginForm.password = ''
  loginError.value = ''
  currentPage.value = 'calendar'
}

async function openSettings() {
  currentPage.value = 'settings'
  settingsError.value = ''
  loadingUsers.value = true
  try {
    users.value = currentUser.value?.role === 'admin' ? await api.listUsers() : currentUser.value ? [currentUser.value] : []
    adminEntries.value = currentUser.value?.role === 'admin'
      ? await api.listEntries(new Date('1970-01-01T00:00:00Z'), new Date('2100-01-01T00:00:00Z'))
      : []
  } catch (error) {
    settingsError.value = error instanceof Error ? error.message : 'Não foi possível carregar as configurações.'
  } finally {
    loadingUsers.value = false
  }
}

async function addProject(project: Project) {
  try {
    projects.value = [...projects.value, await api.createProject({ name: project.name, color: project.color, tint: project.tint })]
  } catch (error) { settingsError.value = error instanceof Error ? error.message : 'Não foi possível criar o projeto.' }
}

async function updateWeeklyGoal(goal: number) {
  if (!currentUser.value) return
  try { weeklyGoal.value = (await api.setWeeklyGoal(goal)).hours } catch (error) { settingsError.value = error instanceof Error ? error.message : 'Não foi possível salvar a meta.' }
}

async function deleteProject(projectId: string) {
  try {
    await api.deleteProject(projectId)
    projects.value = projects.value.filter((project) => project.id !== projectId)
    if (activeProject.value === projectId) activeProject.value = 'all'
  } catch (error) { settingsError.value = error instanceof Error ? error.message : 'Não foi possível arquivar o projeto.' }
}

async function saveUser(input: UserInput) {
  settingsError.value = ''
  try {
    const savedUser = await api.saveUser(input)
    if (currentUser.value?.id === savedUser.id) {
      currentUser.value = savedUser
      if (input.password) {
        currentUser.value = null
        events.value = []
        currentPage.value = 'calendar'
        return
      }
    }
    users.value = await api.listUsers()
  } catch (error) {
    settingsError.value = error instanceof Error ? error.message : 'Não foi possível salvar o usuário.'
  }
}

async function deleteUser(userId: string) {
  if (userId === currentUser.value?.id) return
  settingsError.value = ''
  try {
    await api.deleteUser(userId)
    users.value = await api.listUsers()
  } catch {
    settingsError.value = 'Não foi possível excluir o usuário.'
  }
}

const dashboardEntries = computed(() => adminEntries.value.map((entry) => ({ userId: entry.user_id, projectId: entry.project_id, hours: entry.hours })))

const exportEntries = computed(() => adminEntries.value.map((entry) => ({ userId: entry.user_id, title: entry.title, projectId: entry.project_id, start: entry.starts_at, end: entry.ends_at })))

const visibleWeekEventCount = computed(() => {
  return visibleEvents.value.filter((event) => {
    const start = new Date(event.start as string | Date)
    return start >= visibleRangeStart.value && start < visibleRangeEnd.value
  }).length
})

const calendarOptions = computed<CalendarOptions>(() => ({
  plugins: [timeGridPlugin, interactionPlugin],
  initialView: 'timeGridWeek',
  initialDate: weekStart.value,
  locale: ptBrLocale,
  firstDay: 1,
  allDaySlot: false,
  nowIndicator: true,
  selectable: true,
  editable: true,
  selectMirror: true,
  unselectAuto: true,
  slotMinTime: '06:00:00',
  slotMaxTime: '30:00:00',
  slotDuration: '00:30:00',
  snapDuration: '00:15:00',
  height: '100%',
  expandRows: true,
  headerToolbar: false,
  dayHeaderFormat: { weekday: 'short', day: '2-digit', month: '2-digit', year: 'numeric' },
  dayHeaderContent: (info) => {
    const content = document.createElement('div')
    content.className = 'calendar-day-heading'
    const date = document.createElement('span')
    date.textContent = info.text
    const total = document.createElement('strong')
    total.textContent = `${(dailyHours.value[dateKey(info.date)] ?? 0).toLocaleString('pt-BR', { maximumFractionDigits: 2 })}h`
    content.append(date, total)
    return { domNodes: [content] }
  },
  events: visibleEvents.value,
  select: openNewEvent,
  datesSet: updateWeekLabel,
  eventClick: (info) => openExistingEvent(info.event),
  eventDrop: updateEventTime,
  eventResize: updateEventTime,
}))

function updateWeekLabel(info: DatesSetArg) {
  visibleRangeStart.value = new Date(info.start)
  visibleRangeEnd.value = new Date(info.end)
  weekStart.value = weekStartFor(info.start)
  currentView.value = info.view.type === 'timeGridDay' ? 'timeGridDay' : 'timeGridWeek'
  void loadEntries(info.start, info.end).catch((error) => { formError.value = error instanceof Error ? error.message : 'Não foi possível carregar os registros.' })
  const end = new Date(info.end.getTime() - 24 * 60 * 60 * 1000)
  const formatter = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: '2-digit', year: 'numeric' })
  if (info.view.type === 'timeGridDay') {
    const weekday = new Intl.DateTimeFormat('pt-BR', { weekday: 'long' }).format(info.start)
    weekLabel.value = `${weekday}, ${formatter.format(info.start)}`
    return
  }
  weekLabel.value = `${formatter.format(info.start)} – ${formatter.format(end)}`
}

function openNewEvent(selection: DateSelectArg) {
  editingEventId.value = null
  formError.value = ''
  form.title = ''
  form.projectId = activeProject.value === 'all' ? projects.value[0]?.id ?? '' : activeProject.value
  form.start = hoursInput(selection.start)
  form.end = hoursInput(selection.end)
  form.hours = Math.max(0.25, Math.round((selection.end.getTime() - selection.start.getTime()) / 900_000) / 4)
  dialogOpen.value = true
}

function openExistingEvent(event: ReturnType<CalendarApi['getEventById']> & {}) {
  if (!event?.start || !event.end) return
  editingEventId.value = event.id
  formError.value = ''
  form.title = event.title
  form.projectId = event.extendedProps.projectId ?? projects.value[0]?.id ?? ''
  form.start = hoursInput(event.start)
  form.end = hoursInput(event.end)
  form.hours = Number(event.extendedProps.hours ?? (event.end.getTime() - event.start.getTime()) / 3_600_000)
  dialogOpen.value = true
}

async function updateEventTime(info: { event: { id: string; start: Date | null; end: Date | null; extendedProps: Record<string, unknown>; setExtendedProp: (name: string, value: unknown) => void }; revert: () => void }) {
  if (info.event.start && info.event.end) {
    try {
      const saved = await api.updateEntry(info.event.id, { starts_at: info.event.start.toISOString(), ends_at: info.event.end.toISOString() })
      info.event.setExtendedProp('hours', saved.hours)
      events.value = events.value.map((event) => event.id === saved.id ? toCalendarEvent(saved) : event)
    } catch (error) {
      info.revert()
      formError.value = error instanceof Error ? error.message : 'Não foi possível alterar o horário.'
    }
  }
}

async function saveEvent() {
  const start = parseDateTime(form.start)
  const end = parseDateTime(form.end)
  formError.value = ''
  if (!form.title.trim()) {
    formError.value = 'Informe o nome do evento.'
    return
  }
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime()) || end <= start) {
    formError.value = 'O horário final deve ser posterior ao horário inicial.'
    return
  }
  if (!Number.isFinite(Number(form.hours)) || form.hours <= 0) {
    formError.value = 'Informe um total de horas maior que zero.'
    return
  }

  const project = projects.value.find((item) => item.id === form.projectId)
  if (!project) {
    formError.value = 'Selecione um projeto.'
    return
  }
  try {
    const payload = { title: form.title.trim(), project_id: project.id, starts_at: start.toISOString(), ends_at: end.toISOString() }
    const saved = editingEventId.value ? await api.updateEntry(editingEventId.value, payload) : await api.createEntry(payload)
    const event = toCalendarEvent(saved)
    events.value = editingEventId.value ? events.value.map((item) => item.id === editingEventId.value ? event : item) : [...events.value, event]
    dialogOpen.value = false
  } catch (error) { formError.value = error instanceof Error ? error.message : 'Não foi possível salvar o registro.' }
}

async function removeEvent() {
  if (editingEventId.value) {
    try {
      await api.deleteEntry(editingEventId.value)
      events.value = events.value.filter((event) => event.id !== editingEventId.value)
    } catch (error) { formError.value = error instanceof Error ? error.message : 'Não foi possível excluir o registro.'; return }
  }
  dialogOpen.value = false
}

function moveWeek(direction: number) {
  calendarRef.value?.getApi().incrementDate({ weeks: direction })
}

function goToToday() {
  calendarRef.value?.getApi().today()
}

function changeView(view: 'timeGridWeek' | 'timeGridDay') {
  calendarRef.value?.getApi().changeView(view)
}

function openBlankEvent() {
  const start = new Date()
  start.setMinutes(Math.ceil(start.getMinutes() / 30) * 30, 0, 0)
  const end = new Date(start.getTime() + 60 * 60 * 1000)
  openNewEvent({ start, end } as DateSelectArg)
}
</script>

<template>
  <v-app>
    <main v-if="initializing" class="login-page"><v-progress-circular indeterminate color="#d96446" /></main>
    <main v-else-if="!currentUser" class="login-page">
      <v-card class="login-card" elevation="0">
        <div class="login-brand">
          <span class="brand-mark"><img src="/brand/horasculo-icon.png" alt="" /></span>
          <span>Horasculo<span class="brand-period">.</span></span>
        </div>
        <div class="eyebrow">BEM-VINDO</div>
        <h1>Acesse seu calendário</h1>
        <p class="login-subtitle">Entre para registrar e acompanhar suas horas.</p>
        <form class="login-form" @submit.prevent="submitLogin">
          <v-alert v-if="loginError" type="error" variant="tonal" density="compact">{{ loginError }}</v-alert>
          <v-text-field v-model="loginForm.username" label="Usuário" autocomplete="username" variant="outlined" density="comfortable" autofocus />
          <v-text-field v-model="loginForm.password" label="Senha" type="password" autocomplete="current-password" variant="outlined" density="comfortable" />
          <v-btn class="login-button" type="submit" block :loading="loginLoading">Entrar</v-btn>
        </form>
      </v-card>
    </main>

    <div v-else class="workspace">
      <aside class="sidebar">
        <a class="brand" href="#" aria-label="Horasculo, início">
          <span class="brand-mark"><img src="/brand/horasculo-icon.png" alt="" /></span>
          <span>Horasculo<span class="brand-period">.</span></span>
        </a>

        <v-btn class="create-button" @click="openBlankEvent">
          <v-icon icon="mdi-plus" start /> Novo registro
        </v-btn>

        <div class="sidebar-section">
          <div class="section-label">ESPAÇO DE TRABALHO</div>
          <button class="sidebar-link" :class="{ 'sidebar-link--active': currentPage === 'calendar' }" type="button" @click="currentPage = 'calendar'">
            <v-icon icon="mdi-calendar-month-outline" /> Calendário
          </button>
          <button class="sidebar-link" :class="{ 'sidebar-link--active': currentPage === 'settings' }" type="button" @click="openSettings">
            <v-icon icon="mdi-cog-outline" /> Configurações
          </button>
        </div>

        <div class="sidebar-section projects-section">
          <div class="section-heading">
            <span class="section-label">PROJETOS</span>
            <span class="project-count">{{ projects.length }}</span>
          </div>
          <button
            class="project-link"
            :class="{ 'project-link--active': activeProject === 'all' }"
            type="button"
            @click="activeProject = 'all'"
          >
            <span class="project-dot project-dot--all" /> Todos os projetos
          </button>
          <button
            v-for="project in projects"
            :key="project.id"
            class="project-link"
            :class="{ 'project-link--active': activeProject === project.id }"
            type="button"
            @click="activeProject = project.id"
          >
            <span class="project-dot" :style="{ backgroundColor: project.color }" />
            <span class="project-name">{{ project.name }}</span>
          </button>
        </div>

        <div class="sidebar-bottom">
          <div class="weekly-card">
            <div class="weekly-card-top">
              <span>Esta semana</span>
              <v-icon icon="mdi-chart-box-outline" />
            </div>
            <div class="weekly-total">{{ weekHoursLabel }}<span>h</span></div>
            <div class="weekly-caption">registradas nesta semana</div>
            <div class="weekly-meter"><span :style="{ width: `${Math.min(weekHours / weeklyGoal * 100, 100)}%` }" /></div>
            <div class="weekly-goal">{{ Math.max(weeklyGoal - weekHours, 0).toLocaleString('pt-BR', { maximumFractionDigits: 1 }) }}h para a meta de {{ weeklyGoal.toLocaleString('pt-BR', { maximumFractionDigits: 1 }) }}h</div>
          </div>
          <div class="profile-row">
            <div class="profile-avatar">{{ currentUser.name.charAt(0).toLocaleUpperCase('pt-BR') }}</div>
            <div class="profile-copy"><strong>{{ currentUser.name }}</strong><span>@{{ currentUser.username }}</span></div>
            <v-btn icon="mdi-logout" variant="text" size="small" aria-label="Sair" title="Sair" @click="logout" />
          </div>
        </div>
      </aside>

      <main v-if="currentPage === 'calendar'" class="main-content">
        <header class="topbar">
          <div class="page-heading">
            <span class="eyebrow">REGISTRO DE TEMPO</span>
            <h1>Calendário</h1>
          </div>
          <div class="topbar-actions">
            <div class="week-total-pill"><span class="status-dot" /> {{ weekHoursLabel }}h nesta semana</div>
            <v-btn class="top-create-button" prepend-icon="mdi-plus" @click="openBlankEvent">Novo registro</v-btn>
          </div>
        </header>

        <section class="calendar-panel" aria-label="Calendário semanal">
          <div class="calendar-toolbar">
            <div class="date-navigation">
              <button class="icon-button" type="button" aria-label="Semana anterior" @click="moveWeek(-1)"><v-icon icon="mdi-chevron-left" /></button>
              <button class="icon-button" type="button" aria-label="Próxima semana" @click="moveWeek(1)"><v-icon icon="mdi-chevron-right" /></button>
              <button class="today-button" type="button" @click="goToToday">Hoje</button>
              <h2>{{ weekLabel }}</h2>
            </div>
            <div class="view-switch" aria-label="Visualização do calendário">
              <button :class="{ 'view-switch-active': currentView === 'timeGridWeek' }" type="button" @click="changeView('timeGridWeek')"><v-icon icon="mdi-view-week-outline" /> Semana</button>
              <button :class="{ 'view-switch-active': currentView === 'timeGridDay' }" type="button" @click="changeView('timeGridDay')"><v-icon icon="mdi-view-day-outline" /> Dia</button>
            </div>
          </div>

          <div class="calendar-scroll">
            <FullCalendar ref="calendarRef" :options="calendarOptions" />
          </div>
          <div class="calendar-footer">
            <span><span class="footer-indicator" /> Seus horários aparecem no fuso local</span>
            <span>{{ visibleWeekEventCount }} registros na visualização</span>
          </div>
        </section>
      </main>

      <SettingsView
        v-else
        :projects="projects"
        :users="users"
        :hours-entries="dashboardEntries"
        :export-entries="exportEntries"
        :current-user-id="currentUser.id"
        :loading-users="loadingUsers"
        :error-message="settingsError"
        :weekly-goal="weeklyGoal"
        :is-admin="currentUser.role === 'admin'"
        @add-project="addProject"
        @delete-project="deleteProject"
        @update-weekly-goal="updateWeeklyGoal"
        @save-user="saveUser"
        @delete-user="deleteUser"
      />
    </div>

    <v-dialog v-if="currentUser" v-model="dialogOpen" max-width="520">
      <v-card class="event-dialog">
        <div class="dialog-heading">
          <div>
            <div class="eyebrow">{{ editingEventId ? 'EDITAR REGISTRO' : 'NOVO REGISTRO' }}</div>
            <h2>{{ editingEventId ? 'Detalhes do evento' : 'Adicionar horas' }}</h2>
          </div>
          <v-btn icon="mdi-close" variant="text" aria-label="Fechar" @click="dialogOpen = false" />
        </div>
        <form @submit.prevent="saveEvent">
          <div class="dialog-fields">
            <v-alert v-if="formError" type="error" variant="tonal" density="compact">{{ formError }}</v-alert>
            <v-text-field v-model="form.title" label="Nome do evento" placeholder="Ex.: Planejamento de produto" variant="outlined" density="comfortable" autofocus />
            <v-select v-model="form.projectId" :items="projects" item-title="name" item-value="id" label="Projeto" variant="outlined" density="comfortable" />
            <div class="time-fields">
              <v-text-field v-model="form.start" label="Início" placeholder="DD/MM/AAAA HH:mm" variant="outlined" density="comfortable" />
              <v-text-field v-model="form.end" label="Fim" placeholder="DD/MM/AAAA HH:mm" variant="outlined" density="comfortable" />
            </div>
            <v-text-field v-model.number="form.hours" label="Total de horas trabalhadas" type="number" min="0.25" step="0.25" suffix="h" variant="outlined" density="comfortable" />
          </div>
          <div class="dialog-actions">
            <v-btn v-if="editingEventId" class="delete-button" variant="text" prepend-icon="mdi-trash-can-outline" @click="removeEvent">Excluir</v-btn>
            <span v-else />
            <div class="dialog-actions-right">
              <v-btn variant="text" @click="dialogOpen = false">Cancelar</v-btn>
              <v-btn class="save-button" type="submit">{{ editingEventId ? 'Salvar alterações' : 'Salvar registro' }}</v-btn>
            </div>
          </div>
        </form>
      </v-card>
    </v-dialog>
  </v-app>
</template>
