'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { authApi } from '@/lib/api'
import { LockIcon, FolderIcon, ShieldIcon, EyeIcon, WarningIcon } from '@/components/Icons'
import styles from './setup.module.css'

type Step = 'welcome' | 'create'

export default function SetupPage() {
  const [step, setStep] = useState<Step>('welcome')
  const [accountName, setAccountName] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const [nameAvailable, setNameAvailable] = useState<boolean | null>(null)
  const router = useRouter()
  const initialize = useVaultStore((s) => s.initialize)
  const { sessionToken, _hasHydrated } = useVaultStore()

  const handleNext = async () => {
    if (step === 'welcome') {
      setStep('create')
    }
  }

  const handleNameBlur = async () => {
    if (!accountName.trim()) {
      setNameAvailable(null)
      return
    }
    try {
      const data = await authApi.checkAccountName(accountName.trim())
      setNameAvailable(data.available)
      if (!data.available) {
        setError('This account name is already taken.')
      } else {
        setError('')
      }
    } catch {
      setNameAvailable(null)
    }
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (!accountName.trim()) {
      setError('Account name is required')
      return
    }
    if (nameAvailable === false) {
      setError('This account name is already taken.')
      return
    }
    if (password.length < 8) {
      setError('Password must be at least 8 characters')
      return
    }
    if (password !== confirmPassword) {
      setError('Passwords do not match')
      return
    }

    setLoading(true)
    const result = await initialize(accountName, password)
    if (result.success) {
      router.push('/dashboard')
    } else {
      setError(result.error || 'Failed to initialize vault')
    }
    setLoading(false)
  }

  return (
    <main className={styles.container}>
      <div className={styles.canvas}>
        <div className={styles.gridOverlay} />
        <div className={styles.orb} />
        <div className={styles.orb2} />
      </div>

      <div className={styles.card}>
        {_hasHydrated && sessionToken && (
          <button
            type="button"
            onClick={() => router.push('/dashboard')}
            className={styles.backButton}
          >
            Back to Dashboard
          </button>
        )}
        <div className={styles.logoMark}>
          <svg viewBox="0 0 60 60" className={styles.logoSvg}>
            <defs>
              <linearGradient id="setupGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#60a5fa" />
                <stop offset="50%" stopColor="#3b82f6" />
                <stop offset="100%" stopColor="#2563eb" />
              </linearGradient>
            </defs>
            <circle cx="30" cy="30" r="28" fill="none" stroke="url(#setupGrad)" strokeWidth="2" />
            <circle cx="30" cy="30" r="20" fill="none" stroke="url(#setupGrad)" strokeWidth="1.5" opacity="0.6" />
            <circle cx="30" cy="30" r="12" fill="none" stroke="url(#setupGrad)" strokeWidth="1" opacity="0.4" />
            <circle cx="30" cy="30" r="4" fill="url(#setupGrad)" />
          </svg>
        </div>
        <h1 className={styles.title}>SoloSoul</h1>
        <p className={styles.subtitle}>Digital Twin Engine</p>

        <div className={styles.progress}>
          <div className={`${styles.dot} ${step !== 'welcome' ? styles.active : ''}`} />
        </div>

        {step === 'welcome' && (
          <div className={styles.step}>
            <h2>Welcome</h2>
            <p>
              Your local digital twin and universal identity engine. All your personal data
              is encrypted and stored only on your device.
            </p>

            <div className={styles.features}>
              <div className={styles.feature}>
                <div className={styles.featureIcon}>
                  <LockIcon size={26} />
                </div>
                <div>
                  <strong>Zero-Knowledge Security</strong>
                  <p>Your master password never leaves your device</p>
                </div>
              </div>
              <div className={styles.feature}>
                <div className={styles.featureIcon}>
                  <FolderIcon size={26} />
                </div>
                <div>
                  <strong>One-Time Fill</strong>
                  <p>Fill forms once, reuse everywhere</p>
                </div>
              </div>
              <div className={styles.feature}>
                <div className={styles.featureIcon}>
                  <ShieldIcon size={26} />
                </div>
                <div>
                  <strong>Plugin-Powered</strong>
                  <p>Securely share data with trusted tools</p>
                </div>
              </div>
            </div>

            <div className={styles.centered}>
              <button onClick={handleNext} className={styles.btnPrimary}>
                Get Started
              </button>
            </div>
          </div>
        )}

        {step === 'create' && (
          <form onSubmit={handleSubmit} className={styles.step}>
            <h2>Create Your Vault</h2>
            <p>Set up your account name and master password.</p>

            <div className={styles.field}>
              <label htmlFor="accountName" className={styles.label}>
                Account Name {nameAvailable === false && <span style={{ color: 'var(--color-error)', fontWeight: 'normal' }}>(taken)</span>}
              </label>
              <input
                id="accountName"
                type="text"
                className={`input ${styles.accountInput}`}
                value={accountName}
                onChange={(e) => {
                  setAccountName(e.target.value)
                  setNameAvailable(null)
                }}
                onBlur={handleNameBlur}
                placeholder="e.g., Personal, Work"
                autoFocus
                disabled={loading}
              />
            </div>

            <div className={styles.field}>
              <label htmlFor="password" className={styles.label}>
                Master Password
              </label>
              <div className={styles.passwordWrapper}>
                <input
                  id="password"
                  type="password"
                  className={`input ${styles.passwordInput}`}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="At least 8 characters"
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

            <div className={styles.field}>
              <label htmlFor="confirmPassword" className={styles.label}>
                Confirm Password
              </label>
              <div className={styles.passwordWrapper}>
                <input
                  id="confirmPassword"
                  type="password"
                  className={`input ${styles.passwordInput}`}
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  placeholder="Re-enter your password"
                  disabled={loading}
                />
                <button
                  type="button"
                  className={styles.togglePassword}
                  onMouseDown={() => document.getElementById('confirmPassword')?.setAttribute('type', 'text')}
                  onMouseUp={() => document.getElementById('confirmPassword')?.setAttribute('type', 'password')}
                  onMouseLeave={() => document.getElementById('confirmPassword')?.setAttribute('type', 'password')}
                  disabled={loading}
                >
                  <EyeIcon size={20} />
                </button>
              </div>
            </div>

            {error && <div className={styles.error}>{error}</div>}

            <div className={styles.warning}>
              <div className={styles.warningIcon}>
                <WarningIcon size={32} />
              </div>
              <span>There is no password recovery. If you forget your master password, your data cannot be accessed.</span>
            </div>

            <div className={styles.actions}>
              <button type="button" onClick={() => setStep('welcome')} className={styles.btnSecondary} disabled={loading}>
                Back
              </button>
              <button type="submit" className={styles.btnPrimary} disabled={loading || !accountName.trim() || !password || !confirmPassword}>
                {loading ? 'Creating...' : 'Create Vault'}
              </button>
            </div>
          </form>
        )}
      </div>
    </main>
  )
}
