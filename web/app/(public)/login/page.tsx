'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { authApi } from '@/lib/api'
import { EyeIcon, WarningIcon } from '@/components/Icons'
import styles from './login.module.css'

export default function LoginPage() {
  const [selectedAccount, setSelectedAccount] = useState<string | null>(null)
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const [showCreate, setShowCreate] = useState(false)
  const [newAccountName, setNewAccountName] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmNewPassword, setConfirmNewPassword] = useState('')
  const [createError, setCreateError] = useState('')
  const [createLoading, setCreateLoading] = useState(false)
  const [showAllAccounts, setShowAllAccounts] = useState(false)
  const [checkingName, setCheckingName] = useState(false)
  const [nameAvailable, setNameAvailable] = useState<boolean | null>(null) // null = not checked
  const [returnTo, setReturnTo] = useState<'list' | 'dashboard'>('list')

  const router = useRouter()
  const { unlock, initialize, checkStatus, accounts, listAccounts, currentAccount } = useVaultStore()

  useEffect(() => {
    checkStatus()
    listAccounts()
  }, [checkStatus, listAccounts])

  useEffect(() => {
    // Check URL params on mount to determine if we should show create form
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search)
      if (params.get('create') === 'true') {
        setShowCreate(true)
        setReturnTo('dashboard')
      }
    }
  }, [])

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedAccount) return
    setError('')
    setLoading(true)

    const success = await unlock(selectedAccount, password)
    if (success) {
      router.push('/dashboard')
    } else {
      setError('Invalid master password')
    }
    setLoading(false)
  }

  const handleCreateAccount = async (e: React.FormEvent) => {
    e.preventDefault()

    // Quick validation before doing anything
    if (!newAccountName.trim()) {
      setCreateError('Account name is required')
      return
    }
    if (newPassword.length < 8) {
      setCreateError('Password must be at least 8 characters')
      return
    }
    if (newPassword !== confirmNewPassword) {
      setCreateError('Passwords do not match')
      return
    }

    // If name was checked and not available, show error immediately
    if (nameAvailable === false) {
      setCreateError('This account name is already taken.')
      return
    }

    // Directly attempt creation - backend will reject duplicate names
    setCreateLoading(true)
    const result = await initialize(newAccountName, newPassword)

    if (result.success) {
      // Refresh accounts list before navigating
      await listAccounts()
      router.push('/dashboard')
    } else {
      setCreateError(result.error || 'Failed to create account')
      setCreateLoading(false)
    }
  }

  const handleNameBlur = async () => {
    if (!newAccountName.trim()) {
      setNameAvailable(null)
      return
    }
    setCheckingName(true)
    try {
      const data = await authApi.checkAccountName(newAccountName.trim())
      setNameAvailable(data.available)
      if (!data.available) {
        setCreateError('This account name is already taken.')
      } else {
        setCreateError('')
      }
    } catch {
      setNameAvailable(null)
    }
    setCheckingName(false)
  }

  const formatLastAccessed = (lastAccessed: string) => {
    if (!lastAccessed) return 'Never'
    const date = new Date(lastAccessed)
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))
    if (days === 0) return 'Today'
    if (days === 1) return 'Yesterday'
    if (days < 7) return `${days} days ago`
    return date.toLocaleDateString()
  }

  if (showCreate) {
    return (
      <main className={styles.container}>
        <div className={styles.card}>
          <div className={styles.logo}>
            <h1>SoloSoul</h1>
            <p>Create New Account</p>
          </div>

          <form onSubmit={handleCreateAccount} className={styles.form}>
            <div className={styles.field}>
              <label htmlFor="accountName" className="label">
                Account Name {checkingName && <span className={styles.checking}>Checking...</span>}
              </label>
              <input
                id="accountName"
                type="text"
                className="input"
                value={newAccountName}
                onChange={(e) => {
                  setNewAccountName(e.target.value)
                  setNameAvailable(null) // Reset availability check when typing
                }}
                onBlur={handleNameBlur}
                placeholder="e.g., Personal, Work"
                autoFocus
                disabled={createLoading}
              />
            </div>

            <div className={styles.field}>
              <label htmlFor="newPassword" className="label">
                Master Password
              </label>
              <div className={styles.passwordWrapper}>
                <input
                  id="newPassword"
                  type="password"
                  className={`input ${styles.passwordInput}`}
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="Create a strong password"
                  disabled={createLoading}
                />
                <button
                  type="button"
                  className={styles.togglePassword}
                  onMouseDown={() => document.getElementById('newPassword')?.setAttribute('type', 'text')}
                  onMouseUp={() => document.getElementById('newPassword')?.setAttribute('type', 'password')}
                  onMouseLeave={() => document.getElementById('newPassword')?.setAttribute('type', 'password')}
                  disabled={createLoading}
                >
                  <EyeIcon size={20} />
                </button>
              </div>
            </div>

            <div className={styles.field}>
              <label htmlFor="confirmNewPassword" className="label">
                Confirm Password
              </label>
              <div className={styles.passwordWrapper}>
                <input
                  id="confirmNewPassword"
                  type="password"
                  className={`input ${styles.passwordInput}`}
                  value={confirmNewPassword}
                  onChange={(e) => setConfirmNewPassword(e.target.value)}
                  placeholder="Re-enter your password"
                  disabled={createLoading}
                />
                <button
                  type="button"
                  className={styles.togglePassword}
                  onMouseDown={() => document.getElementById('confirmNewPassword')?.setAttribute('type', 'text')}
                  onMouseUp={() => document.getElementById('confirmNewPassword')?.setAttribute('type', 'password')}
                  onMouseLeave={() => document.getElementById('confirmNewPassword')?.setAttribute('type', 'password')}
                  disabled={createLoading}
                >
                  <EyeIcon size={20} />
                </button>
              </div>
            </div>

            {createError && <div className={styles.error}>{createError}</div>}

            <div className={styles.warning}>
              <div className={styles.warningIcon}>
                <WarningIcon size={28} />
              </div>
              <span>There is no password recovery. If you forget your master password, your data cannot be accessed.</span>
            </div>

            <button
              type="submit"
              className="btn btn-primary"
              disabled={createLoading || !newAccountName || !newPassword || !confirmNewPassword}
            >
              {createLoading ? 'Creating...' : 'Create Account'}
            </button>

            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                if (returnTo === 'dashboard') {
                  router.push('/dashboard')
                } else {
                  setShowCreate(false)
                }
              }}
              disabled={createLoading}
            >
              {returnTo === 'dashboard' ? 'Back to Dashboard' : 'Back to Account List'}
            </button>
          </form>
        </div>
      </main>
    )
  }

  return (
    <main className={styles.container}>
      <div className={styles.card}>
        <div className={styles.logo}>
          <h1>SoloSoul</h1>
          <p>Local Digital Twin Engine</p>
        </div>

        {!selectedAccount ? (
          <div className={styles.accountList}>
            <p className={styles.selectPrompt}>Select an account to unlock</p>
            {accounts.length > 0 ? (
              <div className={styles.accounts}>
                {/* Most recently accessed account first */}
                {(() => {
                  const sortedAccounts = [...accounts].sort((a, b) => {
                    const dateA = new Date(a.last_accessed || 0).getTime()
                    const dateB = new Date(b.last_accessed || 0).getTime()
                    return dateB - dateA
                  })
                  const current = sortedAccounts[0]
                  const otherAccounts = sortedAccounts.slice(1)
                  const displayedAccounts = showAllAccounts ? otherAccounts : otherAccounts.slice(0, 3)
                  const hasMore = otherAccounts.length > 3

                  return (
                    <>
                      <button
                        key={current.id}
                        type="button"
                        className={`${styles.accountButton} ${styles.current}`}
                        onClick={() => setSelectedAccount(current.id)}
                      >
                        <div className={styles.accountIcon}>
                          {current.name.charAt(0).toUpperCase()}
                        </div>
                        <div className={styles.accountInfo}>
                          <div className={styles.accountName}>
                            {current.name}
                            <span className={styles.currentBadge}>Recent</span>
                          </div>
                          <div className={styles.accountMeta}>
                            Last accessed: {formatLastAccessed(current.last_accessed)}
                          </div>
                        </div>
                      </button>

                      {displayedAccounts.map((account) => (
                        <button
                          key={account.id}
                          type="button"
                          className={styles.accountButton}
                          onClick={() => setSelectedAccount(account.id)}
                        >
                          <div className={styles.accountIcon}>
                            {account.name.charAt(0).toUpperCase()}
                          </div>
                          <div className={styles.accountInfo}>
                            <div className={styles.accountName}>{account.name}</div>
                            <div className={styles.accountMeta}>
                              Last accessed: {formatLastAccessed(account.last_accessed)}
                            </div>
                          </div>
                        </button>
                      ))}
                      {hasMore && !showAllAccounts && (
                        <button
                          type="button"
                          className={styles.expandButton}
                          onClick={() => setShowAllAccounts(true)}
                        >
                          Show {otherAccounts.length - 3} more accounts...
                        </button>
                      )}
                    </>
                  )
                })()}
              </div>
            ) : (
              <p className={styles.noAccounts}>No accounts yet</p>
            )}

            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => { setShowCreate(true); setReturnTo('list') }}
            >
              Create New Account
            </button>
          </div>
        ) : (
          <form onSubmit={handleUnlock} className={styles.form}>
            <div className={styles.selectedAccount}>
              <button
                type="button"
                className={styles.backButton}
                onClick={() => {
                  setSelectedAccount(null)
                  setPassword('')
                  setError('')
                }}
              >
                ← Back
              </button>
              <div className={styles.accountBadge}>
                {accounts.find((a) => a.id === selectedAccount)?.name || 'Account'}
              </div>
            </div>

            <div className={styles.field}>
              <label htmlFor="password" className="label">
                Master Password
              </label>
              <div className={styles.passwordWrapper}>
                <input
                  id="password"
                  type="password"
                  className={`input ${styles.passwordInput}`}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Enter your master password"
                  autoFocus
                  disabled={loading}
                />
                <button
                  type="button"
                  className={styles.togglePassword}
                  onMouseDown={() => document.getElementById('password')?.setAttribute('type', 'text')}
                  onMouseUp={() => document.getElementById('password')?.setAttribute('type', 'password')}
                  onMouseLeave={() => document.getElementById('password')?.setAttribute('type', 'password')}
                  disabled={loading}
                >
                  <EyeIcon size={20} />
                </button>
              </div>
            </div>

            {error && <div className={styles.error}>{error}</div>}

            <button
              type="submit"
              className="btn btn-primary"
              disabled={loading || !password}
            >
              {loading ? 'Unlocking...' : 'Unlock'}
            </button>
          </form>
        )}

        <div className={styles.footer}>
          <a href="/setup">First time? Set up your vault</a>
        </div>
      </div>
    </main>
  )
}
