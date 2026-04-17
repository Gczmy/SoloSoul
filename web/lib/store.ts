import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { authApi, AccountInfo } from './api'

interface VaultState {
  isLocked: boolean
  isInitialized: boolean
  sessionToken: string | null
  profileId: string | null
  currentAccount: string | null
  accounts: AccountInfo[]
  _hasHydrated: boolean
  dataVersion: number  // Incremented when data changes (save/delete), pages reload on version bump

  unlock: (accountId: string, password: string) => Promise<boolean>
  lock: () => Promise<void>
  initialize: (accountName: string, password: string) => Promise<{ success: boolean; error?: string }>
  checkStatus: () => Promise<void>
  clearSession: () => void
  listAccounts: () => Promise<void>
  createAccount: (name: string, password: string) => Promise<boolean>
  switchAccount: (accountId: string) => Promise<void>
  deleteAccount: (accountId: string) => Promise<boolean>
  setDefaultAccount: (accountId: string) => Promise<boolean>
  setHasHydrated: (state: boolean) => void
  bumpDataVersion: () => void
  setProfileId: (profileId: string) => void
}

export const useVaultStore = create<VaultState>()(
  persist(
    (set, get) => ({
      isLocked: true,
      isInitialized: false,
      sessionToken: null,
      profileId: null,
      currentAccount: null,
      accounts: [],
      _hasHydrated: false,
      dataVersion: 0,

      bumpDataVersion: () => {
        set({ dataVersion: get().dataVersion + 1 })
      },

      unlock: async (accountId: string, password: string) => {
        try {
          const data = await authApi.unlock(accountId, password)
          if (data.success) {
            set({
              isLocked: false,
              sessionToken: data.session_token || null,
              profileId: data.profile_id || null,
              currentAccount: accountId,
            })
            return true
          }
          return false
        } catch {
          return false
        }
      },

      lock: async () => {
        const { sessionToken } = get()
        if (sessionToken) {
          await authApi.lock(sessionToken)
        }
        set({
          isLocked: true,
          sessionToken: null,
          profileId: null,
        })
      },

      initialize: async (accountName: string, password: string) => {
        try {
          const data = await authApi.setup(accountName, password)
          if (data.success && data.account_id) {
            // Unlock the newly created account to get a session
            const unlockData = await authApi.unlock(data.account_id, password)
            if (unlockData.success) {
              set({
                isInitialized: true,
                isLocked: false,
                sessionToken: unlockData.session_token || null,
                profileId: unlockData.profile_id || null,
                currentAccount: data.account_id,
              })
              // Refresh accounts list so Header shows the new account
              await get().listAccounts()
              return { success: true }
            }
          }
          return { success: false, error: data.error || 'Failed to initialize vault' }
        } catch {
          return { success: false, error: 'Failed to initialize vault' }
        }
      },

      checkStatus: async () => {
        try {
          const data = await authApi.status()
          set((state) => ({
            isInitialized: data.initialized,
            isLocked: data.locked,
            // Preserve locally persisted accounts if backend returns empty or stale data
            accounts: data.accounts?.length
              ? data.accounts
              : (state.accounts.length ? state.accounts : data.accounts || []),
          }))
        } catch {
          // Ignore
        }
      },

      clearSession: () => {
        set({
          sessionToken: null,
          profileId: null,
          isLocked: true,
        })
      },

      listAccounts: async () => {
        try {
          const data = await authApi.listAccounts()
          set({ accounts: data.accounts })
        } catch {
          // Ignore
        }
      },

      createAccount: async (name: string, password: string) => {
        try {
          const data = await authApi.createAccount(name, password)
          if (data.success) {
            await get().listAccounts()
            return true
          }
          return false
        } catch {
          return false
        }
      },

      switchAccount: async (accountId: string) => {
        set({
          isLocked: true,
          sessionToken: null,
          profileId: null,
          currentAccount: accountId,
        })
      },

      deleteAccount: async (accountId: string) => {
        try {
          const data = await authApi.deleteAccount(accountId)
          if (data.success) {
            await get().listAccounts()
            return true
          }
          return false
        } catch {
          return false
        }
      },

      setDefaultAccount: async (accountId: string) => {
        try {
          const data = await authApi.setDefaultAccount(accountId)
          if (data.success) {
            await get().listAccounts()
            return true
          }
          return false
        } catch {
          return false
        }
      },

      setHasHydrated: (state: boolean) => {
        set({ _hasHydrated: state })
      },

      setProfileId: (profileId: string) => {
        set({ profileId })
      },
    }),
    {
      name: 'solosoul-vault',
      partialize: (state) => ({
        isInitialized: state.isInitialized,
        accounts: state.accounts,
        currentAccount: state.currentAccount,
        sessionToken: state.sessionToken,
        profileId: state.profileId,
        dataVersion: state.dataVersion,
        isLocked: state.isLocked,
      }),
      onRehydrateStorage: () => (state) => {
        state?.setHasHydrated(true)
      },
    }
  )
)
