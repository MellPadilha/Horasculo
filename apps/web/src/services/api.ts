export type User = { id: string; username: string; name: string; role: 'admin' | 'user' }
export type UserInput = { id?: string; username: string; name: string; password?: string; role?: 'admin' | 'user' }
export type Project = { id: string; name: string; color: string; tint: string }
export type TimeEntry = {
  id: string
  user_id: string
  project_id: string
  title: string
  starts_at: string
  ends_at: string
  hours: number
  created_at: string
  updated_at: string
}
export type SummaryEntry = { user_id: string; project_id: string; hours: number }

type ApiErrorBody = { message?: string }

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`/api${path}`, {
    ...init,
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json', ...init.headers },
  })
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as ApiErrorBody
    if (response.status === 401) window.dispatchEvent(new Event('horasculo:unauthorized'))
    throw new Error(body.message || 'Não foi possível concluir a operação.')
  }
  if (response.status === 204 || response.headers.get('content-length') === '0') return undefined as T
  return response.json() as Promise<T>
}

export const api = {
  me: () => request<User>('/auth/me'),
  login: (username: string, password: string) => request<User>('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }),
  logout: () => request<void>('/auth/logout', { method: 'POST' }),
  listUsers: () => request<User[]>('/users'),
  saveUser: (input: UserInput) => input.id
    ? request<User>(`/users/${input.id}`, { method: 'PATCH', body: JSON.stringify(input) })
    : request<User>('/users', { method: 'POST', body: JSON.stringify(input) }),
  deleteUser: (id: string) => request<void>(`/users/${id}`, { method: 'DELETE' }),
  listProjects: () => request<Project[]>('/projects'),
  createProject: (input: Omit<Project, 'id'>) => request<Project>('/projects', { method: 'POST', body: JSON.stringify(input) }),
  deleteProject: (id: string) => request<void>(`/projects/${id}`, { method: 'DELETE' }),
  getWeeklyGoal: () => request<{ hours: number }>('/me/weekly-goal'),
  setWeeklyGoal: (hours: number) => request<{ hours: number }>('/me/weekly-goal', { method: 'PUT', body: JSON.stringify({ hours }) }),
  listEntries: (from: Date, to: Date, filters: { projectId?: string; userId?: string } = {}) => {
    const params = new URLSearchParams({ from: from.toISOString(), to: to.toISOString() })
    if (filters.projectId) params.set('project_id', filters.projectId)
    if (filters.userId) params.set('user_id', filters.userId)
    return request<TimeEntry[]>(`/time-entries?${params}`)
  },
  createEntry: (input: { title: string; project_id: string; user_id?: string; starts_at: string; ends_at: string }) => request<TimeEntry>('/time-entries', { method: 'POST', body: JSON.stringify(input) }),
  updateEntry: (id: string, input: Partial<{ title: string; project_id: string; starts_at: string; ends_at: string }>) => request<TimeEntry>(`/time-entries/${id}`, { method: 'PATCH', body: JSON.stringify(input) }),
  deleteEntry: (id: string) => request<void>(`/time-entries/${id}`, { method: 'DELETE' }),
  reportSummary: (from: Date, to: Date, filters: { userId?: string; projectId?: string } = {}) => {
    const params = new URLSearchParams({ from: from.toISOString(), to: to.toISOString() })
    if (filters.userId) params.set('user_id', filters.userId)
    if (filters.projectId) params.set('project_id', filters.projectId)
    return request<SummaryEntry[]>(`/reports/summary?${params}`)
  },
  downloadReport: async (from: Date, to: Date) => {
    const params = new URLSearchParams({ from: from.toISOString(), to: to.toISOString() })
    const response = await fetch(`/api/reports/time-entries.xlsx?${params}`, { credentials: 'same-origin' })
    if (response.status === 401) window.dispatchEvent(new Event('horasculo:unauthorized'))
    if (!response.ok) {
      const body = await response.json().catch(() => ({})) as ApiErrorBody
      throw new Error(body.message || 'Não foi possível exportar os registros.')
    }
    return response.blob()
  },
}
