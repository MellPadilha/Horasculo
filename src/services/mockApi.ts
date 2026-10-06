export type MockUser = {
  id: string
  username: string
  name: string
}

export type MockUserInput = {
  id?: string
  username: string
  name: string
  password?: string
}

type MockAccount = MockUser & {
  password: string
}

const accountsStorageKey = 'horas.mock.accounts'
const defaultAccounts: MockAccount[] = [
  { id: 'demo', username: 'demo', name: 'Demo', password: '12345' },
]

const wait = (milliseconds: number) => new Promise((resolve) => setTimeout(resolve, milliseconds))

function loadAccounts(): MockAccount[] {
  try {
    const stored = localStorage.getItem(accountsStorageKey)
    return stored ? JSON.parse(stored) as MockAccount[] : [...defaultAccounts]
  } catch {
    return [...defaultAccounts]
  }
}

function saveAccounts(accounts: MockAccount[]) {
  localStorage.setItem(accountsStorageKey, JSON.stringify(accounts))
}

const publicUser = ({ password: _password, ...user }: MockAccount): MockUser => user

export async function loginRequest(username: string, password: string): Promise<MockUser> {
  await wait(450)

  const normalizedUsername = username.trim().toLocaleLowerCase('pt-BR')
  const account = loadAccounts().find((item) => item.username === normalizedUsername && item.password === password)

  if (!account) {
    throw new Error('Usuário ou senha inválidos.')
  }

  return publicUser(account)
}

export async function listUsersRequest(): Promise<MockUser[]> {
  await wait(180)
  return loadAccounts().map(publicUser)
}

export async function saveUserRequest(input: MockUserInput): Promise<MockUser> {
  await wait(250)
  const accounts = loadAccounts()
  const username = input.username.trim().toLocaleLowerCase('pt-BR')
  const duplicate = accounts.some((item) => item.username === username && item.id !== input.id)
  if (duplicate) throw new Error('Este nome de usuário já está em uso.')

  if (input.id) {
    const index = accounts.findIndex((item) => item.id === input.id)
    if (index < 0) throw new Error('Usuário não encontrado.')
    accounts[index] = {
      ...accounts[index],
      name: input.name.trim(),
      username,
      ...(input.password ? { password: input.password } : {}),
    }
    saveAccounts(accounts)
    return publicUser(accounts[index])
  }

  if (!input.password) throw new Error('Informe uma senha para o novo usuário.')
  const account: MockAccount = {
    id: `user-${Date.now()}`,
    name: input.name.trim(),
    username,
    password: input.password,
  }
  accounts.push(account)
  saveAccounts(accounts)
  return publicUser(account)
}

export async function deleteUserRequest(userId: string): Promise<void> {
  await wait(200)
  saveAccounts(loadAccounts().filter((item) => item.id !== userId))
}
