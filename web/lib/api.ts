// API client for communicating with SoloSoul Go backend

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
  body?: unknown
  token?: string
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }

  if (options.token) {
    headers['Authorization'] = `Bearer ${options.token}`
  }

  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), 10000)

  try {
    const res = await fetch(`${API_BASE}${path}`, {
      method: options.method || 'GET',
      headers,
      body: options.body ? JSON.stringify(options.body) : undefined,
      signal: controller.signal,
    })
    clearTimeout(timeout)

    if (!res.ok) {
      const err = await res.text().catch(() => `HTTP ${res.status}`)
      throw new Error(err)
    }

    return res.json()
  } catch (e) {
    clearTimeout(timeout)
    if (e instanceof Error && e.name === 'AbortError') {
      throw new Error('Request timed out. Is the backend running?')
    }
    throw e
  }
}

// Account types
export interface AccountInfo {
  id: string
  name: string
  created_at: string
  last_accessed: string
}

// Auth API
export const authApi = {
  unlock: (accountId: string, password: string) =>
    request<{ success: boolean; session_token?: string; profile_id?: string; error?: string }>(
      '/api/auth/unlock',
      { method: 'POST', body: { account_id: accountId, master_password: password } }
    ),

  lock: (token?: string) =>
    request<{ success: boolean }>('/api/auth/lock', { method: 'POST', token }),

  changePassword: (oldPassword: string, newPassword: string, token: string) =>
    request<{ success: boolean; error?: string }>('/api/auth/password', {
      method: 'POST',
      body: { old_password: oldPassword, new_password: newPassword },
      token,
    }),

  setup: (accountName: string, password: string) =>
    request<{ success: boolean; account_id?: string; error?: string }>('/api/auth/setup', {
      method: 'POST',
      body: { account_name: accountName, master_password: password },
    }),

  status: () =>
    request<{
      initialized: boolean
      locked: boolean
      accounts: AccountInfo[]
      current_account: string | null
    }>('/api/auth/status'),

  // Account management
  listAccounts: () =>
    request<{ accounts: AccountInfo[]; default_account: string }>('/api/accounts'),

  checkAccountName: (name: string) =>
    request<{ available: boolean }>(`/api/accounts/check?name=${encodeURIComponent(name)}`),

  createAccount: (accountName: string, password: string) =>
    request<{ success: boolean; account_id?: string; error?: string }>('/api/accounts', {
      method: 'POST',
      body: { account_name: accountName, master_password: password },
    }),

  deleteAccount: (accountId: string) =>
    request<{ success: boolean; error?: string }>(`/api/accounts/${accountId}`, {
      method: 'DELETE',
    }),

  setDefaultAccount: (accountId: string) =>
    request<{ success: boolean }>(`/api/accounts/${accountId}/default`, {
      method: 'PUT',
    }),
}

// Profile API
export interface Profile {
  profile_id: string
  version: string
  created_at: string
  updated_at: string
  identity?: {
    full_name?: { full_name?: string; given_name?: string; family_name?: string }
    date_of_birth?: { year: number; month: number; day: number }
    gender?: string
    contact?: { emails?: Array<{ value: string; label?: string }>; phones?: Array<{ value: string; label?: string }> }
    primary_address?: { street?: string; city?: string; state?: string; postal_code?: string; country?: string }
  }
  travel?: {
    primary_passport?: {
      number?: string
      country?: string
      nationality?: string
      expiry_date?: { year: number; month: number; day: number }
    }
  }
  financial?: {
    bank_accounts?: Array<{ bank_name?: string; account_number?: string; currency?: string }>
  }
  professional?: {
    education?: Array<{ institution?: string; degree?: string; field?: string }>
    employments?: Array<{ company?: string; title?: string; current?: boolean }>
  }
  preferences?: {
    meal_preference?: string
    seat_preference?: string
  }
}

export const profileApi = {
  get: (profileId: string, token?: string) =>
    request<{ success: boolean; profile?: Profile; error?: string }>(
      `/api/profile/${profileId}`,
      { token }
    ),

  update: (profile: Profile, token?: string) =>
    request<{ success: boolean; error?: string }>('/api/profile', {
      method: 'PUT',
      token,
      body: { profile },
    }),

  list: (token?: string) =>
    request<{ profile_ids: string[] }>('/api/profile', { token }),

  validate: (profile: Profile, token?: string) =>
    request<{ valid: boolean; errors: Array<{ field: string; message: string }> }>(
      '/api/profile/validate',
      { method: 'POST', token, body: { profile } }
    ),

  delete: (profileId: string, token?: string) =>
    request<{ success: boolean; error?: string }>(`/api/profile/${profileId}`, {
      method: 'DELETE',
      token,
    }),
}

// Plugin API
export interface PluginManifest {
  id: string
  name: string
  version: string
  description?: string
  publisher?: string
  required_fields?: string[]
  optional_fields?: string[]
  requires_consent?: boolean
}

export interface ConsentSession {
  session_id: string
  plugin_id: string
  fields: string[]
  created_at: string
  expires_at: string
  revoked: boolean
}

export const pluginApi = {
  list: (token?: string) =>
    request<{ plugins: Array<{ id: string; name: string; version: string; is_approved: boolean }> }>(
      '/api/plugins',
      { token }
    ),

  getManifest: (pluginId: string, token?: string) =>
    request<{ manifest: PluginManifest }>(`/api/plugins/${pluginId}/manifest`, { token }),

  requestConsent: (pluginId: string, fields: string[], token?: string) =>
    request<{ request_id: string; status: string; error?: string }>(
      `/api/plugins/${pluginId}/consent/request`,
      { method: 'POST', token, body: { requested_fields: fields } }
    ),

  grantConsent: (requestId: string, fields: string[], validityHours: number, token?: string) =>
    request<{ success: boolean; session_id?: string; expires_at?: string; error?: string }>(
      '/api/plugins/consent/grant',
      {
        method: 'POST',
        token,
        body: { request_id: requestId, authorized_fields: fields, validity_hours: validityHours },
      }
    ),

  revokeConsent: (sessionId: string, token?: string) =>
    request<{ success: boolean }>(`/api/plugins/sessions/${sessionId}`, {
      method: 'DELETE',
      token,
    }),

  listSessions: (pluginId: string, token?: string) =>
    request<{ sessions: ConsentSession[] }>(`/api/plugins/${pluginId}/sessions`, { token }),
}

// OCR API
export interface OCRJob {
  id: string
  status: 'pending' | 'processing' | 'completed' | 'failed'
  document_type: string
  result?: {
    fields: Array<{ key: string; value: string; confidence: number }>
  }
}

export const ocrApi = {
  submitJob: (imageData: string, documentType: string, token?: string) =>
    request<{ job_id: string; status: string }>('/api/ocr/jobs', {
      method: 'POST',
      token,
      body: { image_data: imageData, document_type: documentType },
    }),

  getResult: (jobId: string, token?: string) =>
    request<{ success: boolean; status: string; fields?: Array<{ key: string; value: string; confidence: number }>; error?: string }>(
      `/api/ocr/jobs/${jobId}`,
      { token }
    ),
}
