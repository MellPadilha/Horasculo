<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import FullCalendar from '@fullcalendar/vue3'
import timeGridPlugin from '@fullcalendar/timegrid'
import interactionPlugin from '@fullcalendar/interaction'
import ptBrLocale from '@fullcalendar/core/locales/pt-br'
import type { CalendarApi, CalendarOptions, DateSelectArg, DatesSetArg, EventInput } from '@fullcalendar/core'
import SettingsView from './components/SettingsView.vue'
import { deleteUserRequest, listUsersRequest, loginRequest, saveUserRequest, type MockUser, type MockUserInput } from './services/mockApi'

type Project = {
  id: string
  name: string
  color: string
  tint: string
}

const defaultProjects: Project[] = [
  { id: 'aurora', name: 'Aurora Studio', color: '#c95f45', tint: '#f7c8bb' },
  { id: 'northstar', name: 'Northstar', color: '#267d69', tint: '#bfe4d8' },
  { id: 'fieldnotes', name: 'Fieldnotes', color: '#4f70b5', tint: '#c9d8f5' },
  { id: 'internal', name: 'Interno', color: '#a87825', tint: '#f2dead' },
]

const projectsStorageKey = 'horas.projects'

function loadProjects(): Project[] {
  try {
    const stored = localStorage.getItem(projectsStorageKey)
    const savedProjects = stored ? JSON.parse(stored) as Project[] : defaultProjects
    return savedProjects.map((project) => ({
      ...project,
      tint: /^#[\da-f]{8}$/i.test(project.tint) ? opaqueHex(project.tint) : project.tint,
    }))
  } catch {
    return defaultProjects
  }
}

function opaqueHex(hex: string) {
  const red = Number.parseInt(hex.slice(1, 3), 16)
  const green = Number.parseInt(hex.slice(3, 5), 16)
  const blue = Number.parseInt(hex.slice(5, 7), 16)
  const alpha = Number.parseInt(hex.slice(7, 9), 16) / 255
  const blend = (channel: number) => Math.round(channel * alpha + 255 * (1 - alpha)).toString(16).padStart(2, '0')
  return `#${blend(red)}${blend(green)}${blend(blue)}`
}

const projects = ref<Project[]>(loadProjects())
watch(projects, (value) => localStorage.setItem(projectsStorageKey, JSON.stringify(value)), { deep: true })

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

function demoEvent(day: number, startHour: number, startMinute: number, duration: number, projectId: string, title: string, hours: number): EventInput {
  const start = new Date(weekStart.value)
  start.setDate(start.getDate() + day)
  start.setHours(startHour, startMinute, 0, 0)
  const end = new Date(start.getTime() + duration * 60_000)
  const project = projects.value.find((item) => item.id === projectId) ?? defaultProjects[0]
  return {
    id: `${projectId}-${day}-${startHour}`,
    title,
    start,
    end,
    backgroundColor: project.tint,
    borderColor: project.color,
    textColor: '#252923',
    extendedProps: { projectId, hours },
  }
}

const demoEvents: EventInput[] = [
  demoEvent(0, 9, 0, 90, 'aurora', 'Direção de arte', 1.5),
  demoEvent(0, 13, 30, 120, 'northstar', 'Revisão de produto', 2),
  demoEvent(1, 10, 0, 120, 'fieldnotes', 'Pesquisa de conteúdo', 2),
  demoEvent(1, 14, 0, 90, 'aurora', 'Apresentação da marca', 1.5),
  demoEvent(2, 9, 30, 150, 'northstar', 'Planejamento da sprint', 2.5),
  demoEvent(3, 11, 0, 60, 'internal', 'Reunião semanal', 1),
  demoEvent(3, 14, 0, 120, 'fieldnotes', 'Protótipo de navegação', 2),
  demoEvent(4, 9, 0, 90, 'aurora', 'Ajustes finais', 1.5),
]

const sessionStorageKey = 'horas.session'
const weeklyGoalStorageKey = (userId: string) => `horas.weekly-goal.${userId}`

function loadWeeklyGoal(userId?: string) {
  if (!userId) return 32
  const savedGoal = Number(localStorage.getItem(weeklyGoalStorageKey(userId)))
  return Number.isFinite(savedGoal) && savedGoal > 0 ? savedGoal : 32
}

function loadSession(): MockUser | null {
  try {
    const session = localStorage.getItem(sessionStorageKey)
    return session ? JSON.parse(session) as MockUser : null
  } catch {
    return null
  }
}

const currentUser = ref<MockUser | null>(loadSession())
const weeklyGoal = ref(loadWeeklyGoal(currentUser.value?.id))
const currentPage = ref<'calendar' | 'settings'>('calendar')
const users = ref<MockUser[]>([])
const loadingUsers = ref(false)
const settingsError = ref('')
const loginLoading = ref(false)
const loginError = ref('')
const loginForm = reactive({ username: '', password: '' })

const eventsStorageKey = (userId: string) => `horas.calendar.events.${userId}`

function loadEvents(userId?: string): EventInput[] {
  if (!userId) return []
  try {
    const storedEvents = localStorage.getItem(eventsStorageKey(userId))
    return storedEvents ? JSON.parse(storedEvents) as EventInput[] : demoEvents
  } catch {
    return demoEvents
  }
}

function loadStoredEvents(userId: string): EventInput[] {
  try {
    const storedEvents = localStorage.getItem(eventsStorageKey(userId))
    return storedEvents ? JSON.parse(storedEvents) as EventInput[] : []
  } catch {
    return []
  }
}

const events = ref<EventInput[]>(loadEvents(currentUser.value?.id))

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

watch(events, (value) => {
  if (!currentUser.value) return
  try {
    localStorage.setItem(eventsStorageKey(currentUser.value.id), JSON.stringify(value))
  } catch {
    // A agenda continua funcional em memória caso o armazenamento esteja indisponível.
  }
}, { deep: true })

async function submitLogin() {
  loginError.value = ''
  if (!loginForm.username.trim() || !loginForm.password) {
    loginError.value = 'Informe usuário e senha.'
    return
  }

  loginLoading.value = true
  try {
    const user = await loginRequest(loginForm.username, loginForm.password)
    currentUser.value = user
    weeklyGoal.value = loadWeeklyGoal(user.id)
    events.value = loadEvents(user.id)
    localStorage.setItem(sessionStorageKey, JSON.stringify(user))
    loginForm.password = ''
  } catch (error) {
    loginError.value = error instanceof Error ? error.message : 'Não foi possível entrar.'
  } finally {
    loginLoading.value = false
  }
}

function logout() {
  localStorage.removeItem(sessionStorageKey)
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
    users.value = await listUsersRequest()
  } finally {
    loadingUsers.value = false
  }
}

function addProject(project: Project) {
  projects.value = [...projects.value, project]
}

function updateWeeklyGoal(goal: number) {
  if (!currentUser.value) return
  weeklyGoal.value = goal
  localStorage.setItem(weeklyGoalStorageKey(currentUser.value.id), String(goal))
}

function deleteProject(projectId: string) {
  projects.value = projects.value.filter((project) => project.id !== projectId)
  if (activeProject.value === projectId) activeProject.value = 'all'
}

async function saveUser(input: MockUserInput) {
  settingsError.value = ''
  try {
    const savedUser = await saveUserRequest(input)
    users.value = await listUsersRequest()
    if (currentUser.value?.id === savedUser.id) {
      currentUser.value = savedUser
      localStorage.setItem(sessionStorageKey, JSON.stringify(savedUser))
    }
  } catch (error) {
    settingsError.value = error instanceof Error ? error.message : 'Não foi possível salvar o usuário.'
  }
}

async function deleteUser(userId: string) {
  if (userId === currentUser.value?.id) return
  settingsError.value = ''
  try {
    await deleteUserRequest(userId)
    users.value = await listUsersRequest()
    localStorage.removeItem(eventsStorageKey(userId))
  } catch {
    settingsError.value = 'Não foi possível excluir o usuário.'
  }
}

const dashboardEntries = computed(() => users.value.flatMap((user) => {
  const userEvents = user.id === currentUser.value?.id ? events.value : loadStoredEvents(user.id)
  return userEvents.map((event) => ({
    userId: user.id,
    projectId: String(event.extendedProps?.projectId ?? ''),
    hours: Number(event.extendedProps?.hours ?? 0),
  }))
}))

const exportEntries = computed(() => users.value.flatMap((user) => {
  const userEvents = user.id === currentUser.value?.id ? events.value : loadStoredEvents(user.id)
  return userEvents.map((event) => ({
    userId: user.id,
    title: String(event.title ?? ''),
    start: new Date(event.start as string | Date).toISOString(),
    end: new Date(event.end as string | Date).toISOString(),
  }))
}))

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

function updateEventTime(info: { event: { id: string; start: Date | null; end: Date | null; extendedProps: Record<string, unknown>; setExtendedProp: (name: string, value: unknown) => void } }) {
  if (info.event.start && info.event.end) {
    const hours = Math.round((info.event.end.getTime() - info.event.start.getTime()) / 900_000) / 4
    info.event.setExtendedProp('hours', hours)
    events.value = events.value.map((event) => event.id === info.event.id
      ? {
          ...event,
          start: new Date(info.event.start!),
          end: new Date(info.event.end!),
          extendedProps: { ...event.extendedProps, hours },
        }
      : event)
  }
}

function saveEvent() {
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
  const event: EventInput = {
    id: editingEventId.value ?? `event-${Date.now()}`,
    title: form.title.trim(),
    start,
    end,
    backgroundColor: project.tint,
    borderColor: project.color,
    textColor: '#252923',
    extendedProps: { projectId: project.id, hours: Number(form.hours) },
  }
  events.value = editingEventId.value
    ? events.value.map((item) => item.id === editingEventId.value ? event : item)
    : [...events.value, event]
  dialogOpen.value = false
}

function removeEvent() {
  if (editingEventId.value) {
    events.value = events.value.filter((event) => event.id !== editingEventId.value)
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
    <main v-if="!currentUser" class="login-page">
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
        <div class="demo-access">
          <strong>Acesso de demonstração</strong>
          <span><code>demo</code> / <code>12345</code></span>
        </div>
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
