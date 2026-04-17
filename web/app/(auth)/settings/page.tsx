'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { authApi } from '@/lib/api'
import Header from '../Header'
import { ChevronIcon, EyeIcon, WarningIcon, TrashIcon } from '@/components/Icons'
import styles from './settings.module.css'

export default function SettingsPage() {
  const [changingPassword, setChangingPassword] = useState(false)
  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)
  const [showPasswordForm, setShowPasswordForm] = useState(false)
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null)
  const [showDeleteModal, setShowDeleteModal] = useState(false)
  const [deletingAccount, setDeletingAccount] = useState<typeof accounts[0] | null>(null)

  const { lock, accounts, currentAccount, deleteAccount, sessionToken, listAccounts, checkStatus } = useVaultStore()
  const router = useRouter()

  // Refresh accounts when page becomes visible again (e.g., returning from login)
  useEffect(() => {
    const refresh = async () => {
      await listAccounts()
    }

    // Refresh on mount
    refresh()

    // Refresh when page becomes visible
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        refresh()
      }
    }
    document.addEventListener('visibilitychange', handleVisibilityChange)

    // Refresh on focus (when user comes back to the tab)
    window.addEventListener('focus', refresh)

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
      window.removeEventListener('focus', refresh)
    }
  }, [listAccounts])

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault()
    if (newPassword !== confirmPassword) {
      setMessage({ type: 'error', text: 'Passwords do not match' })
      return
    }
    if (newPassword.length < 8) {
      setMessage({ type: 'error', text: 'Password must be at least 8 characters' })
      return
    }
    if (!sessionToken) {
      setMessage({ type: 'error', text: 'Session expired. Please log in again.' })
      return
    }
    setChangingPassword(true)
    setMessage(null)
    try {
      const result = await authApi.changePassword(currentPassword, newPassword, sessionToken)
      if (result.success) {
        setMessage({ type: 'success', text: 'Password changed successfully' })
        setCurrentPassword('')
        setNewPassword('')
        setConfirmPassword('')
      } else {
        setMessage({ type: 'error', text: result.error || 'Current password is incorrect' })
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to change password'
      setMessage({ type: 'error', text: msg })
    } finally {
      setChangingPassword(false)
    }
  }

  const handleDeleteClick = (accountId: string) => {
    // Can only delete the currently logged-in account
    if (accountId !== currentAccount) {
      return
    }
    const account = accounts.find(a => a.id === accountId)
    if (account) {
      setDeletingAccount(account)
      setShowDeleteModal(true)
    }
  }

  const handleConfirmDelete = async () => {
    if (!deletingAccount) return

    const isLastAccount = accounts.length === 1
    const success = await deleteAccount(deletingAccount.id)
    if (success) {
      setShowDeleteModal(false)
      setDeletingAccount(null)
      await lock()
      if (isLastAccount) {
        router.push('/setup')
      } else {
        router.push('/login')
      }
    }
  }

  const handleCancelDelete = () => {
    setShowDeleteModal(false)
    setDeletingAccount(null)
  }

  const formatDate = (dateStr: string) => {
    if (!dateStr) return 'Never'
    const d = new Date(dateStr)
    const pad = (n: number) => n.toString().padStart(2, '0')
    const yyyy = d.getFullYear()
    const mm = pad(d.getMonth() + 1)
    const dd = pad(d.getDate())
    const hh = pad(d.getHours())
    const min = pad(d.getMinutes())
    return `${yyyy}/${mm}/${dd} ${hh}:${min}`
  }

  const sortedAccounts = [...accounts].sort((a, b) => {
    // Current account always first
    if (a.id === currentAccount) return -1
    if (b.id === currentAccount) return 1
    // Then by last_accessed descending (most recent first)
    const aTime = a.last_accessed ? new Date(a.last_accessed).getTime() : 0
    const bTime = b.last_accessed ? new Date(b.last_accessed).getTime() : 0
    return bTime - aTime
  })

  return (
    <main className={styles.container}>
      <Header />

      <div className={styles.content}>
        <section className={styles.section}>
          <h3>Accounts</h3>
          <div className={styles.card}>
            <h4>Manage Accounts</h4>
            <p className={styles.hint}>
              Each account has its own encrypted vault with separate master password.
            </p>

            <div className={styles.accountList}>
              {sortedAccounts.map((account) => (
                <div
                  key={account.id}
                  className={`${styles.accountItem} ${account.id === currentAccount ? styles.isCurrent : ''}`}
                >
                  <div className={styles.accountIcon}>
                    {account.name.charAt(0).toUpperCase()}
                  </div>
                  <div className={styles.accountDetails}>
                    <div className={styles.accountName}>
                      {account.name}
                      {account.id === currentAccount && (
                        <span className={styles.currentBadge}>Current</span>
                      )}
                    </div>
                    <div className={styles.accountMeta}>
                      Created {formatDate(account.created_at)}
                      <span>·</span>
                      Last accessed {formatDate(account.last_accessed)}
                    </div>
                  </div>
                  <div className={styles.accountActions}>
                    {account.id === currentAccount && (
                      <button
                        onClick={() => handleDeleteClick(account.id)}
                        className={`${styles.actionButton} ${styles.danger}`}
                      >
                        Delete
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>

            <button
              onClick={() => router.push('/login?create=true')}
              className={styles.createButton}
            >
              Create New Account
            </button>
          </div>
        </section>

        <section className={styles.section}>
          <h3>Security</h3>
          <div className={styles.card}>
            <button
              type="button"
              className={styles.collapseHeader}
              onClick={() => setShowPasswordForm(!showPasswordForm)}
            >
              <div className={styles.collapseTitle}>
                <h4>Change Master Password</h4>
                <p>Update your vault master password</p>
              </div>
              <span className={`${styles.collapseIcon} ${showPasswordForm ? styles.open : ''}`}>
                <ChevronIcon direction="right" open={showPasswordForm} size={14} />
              </span>
            </button>

            <div className={`${styles.collapseContent} ${showPasswordForm ? styles.open : ''}`}>
              <div>
                <form onSubmit={handleChangePassword}>
                <div className={styles.field}>
                  <label className="label">Current Password</label>
                  <div className={styles.passwordWrapper}>
                    <input
                      type="password"
                      id="currentPassword"
                      className={`input ${styles.passwordInput}`}
                      value={currentPassword}
                      onChange={(e) => setCurrentPassword(e.target.value)}
                    />
                    <button
                      type="button"
                      className={styles.togglePassword}
                      onMouseDown={() => document.getElementById('currentPassword')?.setAttribute('type', 'text')}
                      onMouseUp={() => document.getElementById('currentPassword')?.setAttribute('type', 'password')}
                      onMouseLeave={() => document.getElementById('currentPassword')?.setAttribute('type', 'password')}
                    >
                      <EyeIcon size={20} />
                    </button>
                  </div>
                </div>
                <div className={styles.field}>
                  <label className="label">New Password</label>
                  <div className={styles.passwordWrapper}>
                    <input
                      type="password"
                      id="newPassword"
                      className={`input ${styles.passwordInput}`}
                      value={newPassword}
                      onChange={(e) => setNewPassword(e.target.value)}
                    />
                    <button
                      type="button"
                      className={styles.togglePassword}
                      onMouseDown={() => document.getElementById('newPassword')?.setAttribute('type', 'text')}
                      onMouseUp={() => document.getElementById('newPassword')?.setAttribute('type', 'password')}
                      onMouseLeave={() => document.getElementById('newPassword')?.setAttribute('type', 'password')}
                    >
                      <EyeIcon size={20} />
                    </button>
                  </div>
                </div>
                <div className={styles.field}>
                  <label className="label">Confirm New Password</label>
                  <div className={styles.passwordWrapper}>
                    <input
                      type="password"
                      id="confirmNewPassword"
                      className={`input ${styles.passwordInput}`}
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                    />
                    <button
                      type="button"
                      className={styles.togglePassword}
                      onMouseDown={() => document.getElementById('confirmNewPassword')?.setAttribute('type', 'text')}
                      onMouseUp={() => document.getElementById('confirmNewPassword')?.setAttribute('type', 'password')}
                      onMouseLeave={() => document.getElementById('confirmNewPassword')?.setAttribute('type', 'password')}
                    >
                      <EyeIcon size={20} />
                    </button>
                  </div>
                </div>
                {message && (
                  <div className={`${styles.message} ${styles[message.type]}`}>{message.text}</div>
                )}
                <button type="submit" className="btn btn-primary" disabled={changingPassword}>
                  {changingPassword ? 'Changing...' : 'Change Password'}
                </button>
              </form>
              </div>
            </div>
          </div>
        </section>

        <section className={styles.section}>
          <h3>Data</h3>
          <div className={`${styles.card} ${styles.exportCard}`}>
            <div className={styles.exportInfo}>
              <h4>Export Data</h4>
              <p>Export your profile data as an encrypted backup file.</p>
            </div>
            <button className="btn btn-secondary">Export Backup</button>
          </div>
        </section>

        <section className={styles.section}>
          <h3>About</h3>
          <div className={styles.card}>
            <h4>SoloSoul</h4>
            <p className={styles.version}>Version 1.0.0</p>
            <p className={styles.description}>
              Your local digital twin and universal identity engine.
              All data is stored encrypted on your device.
            </p>
          </div>
        </section>
      </div>

      {/* Delete Confirmation Modal */}
      {showDeleteModal && deletingAccount && (
        <div className={styles.modalOverlay} onClick={handleCancelDelete}>
          <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
            <div className={styles.modalIcon}>
              <WarningIcon size={32} />
            </div>
            <h3 className={styles.modalTitle}>Delete Account</h3>
            <p className={styles.modalText}>
              Are you sure you want to delete <strong>{deletingAccount.name}</strong>?
            </p>
            <p className={styles.modalWarning}>
              All vault data, profiles, and saved information will be permanently erased. This action cannot be undone.
            </p>
            <div className={styles.modalActions}>
              <button onClick={handleCancelDelete} className={styles.modalCancel}>
                Cancel
              </button>
              <button onClick={handleConfirmDelete} className={styles.modalConfirm}>
                Delete Account
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  )
}
