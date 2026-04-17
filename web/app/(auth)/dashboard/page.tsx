'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { Profile, profileApi } from '@/lib/api'
import Header from '../Header'
import styles from './dashboard.module.css'

export default function DashboardPage() {
  const [profile, setProfile] = useState<Profile | null>(null)
  const [loading, setLoading] = useState(true)
  const router = useRouter()
  const { sessionToken, currentAccount, profileId, dataVersion } = useVaultStore()

  useEffect(() => {
    // Redirect to login if session is invalid
    const { sessionToken, currentAccount } = useVaultStore.getState()
    if (!sessionToken || !currentAccount) {
      router.push('/login')
    }
    setLoading(false)
  }, [router])

  // Fetch profile data on mount and when dataVersion changes
  useEffect(() => {
    const loadProfile = async () => {
      const { profileId, sessionToken } = useVaultStore.getState()
      if (!profileId || !sessionToken) return

      try {
        const result = await profileApi.get(profileId, sessionToken)
        if (result.success && result.profile) {
          setProfile(result.profile)
        }
      } catch {
        // Failed to load profile
      }
    }

    loadProfile()
  }, [dataVersion])

  const completeness = calculateCompleteness(profile)

  return (
    <main className={styles.container}>
      <Header />
      <div className={styles.content}>
        <div className={styles.welcome}>
          <h2>Welcome back</h2>
          <p>Your local digital twin is ready.</p>
        </div>

        <div className={styles.stats}>
          <div className={`card ${styles.statCard}`}>
            <div className={styles.statValue}>{completeness}%</div>
            <div className={styles.statLabel}>Profile Complete</div>
            <div className={styles.progressBar}>
              <div className={styles.progressFill} style={{ width: `${completeness}%` }} />
            </div>
          </div>

          <div className={`card ${styles.statCard}`}>
            <div className={styles.statValue}>{profile ? '1' : '0'}</div>
            <div className={styles.statLabel}>Profiles</div>
          </div>

          <div className={`card ${styles.statCard}`}>
            <div className={styles.statValue}>0</div>
            <div className={styles.statLabel}>Documents</div>
          </div>

          <div className={`card ${styles.statCard}`}>
            <div className={styles.statValue}>0</div>
            <div className={styles.statLabel}>Active Sessions</div>
          </div>
        </div>

        <div className={styles.quickActions}>
          <h3>Quick Actions</h3>
          <div className={styles.actionGrid}>
            <a href="/profile" className={styles.actionCard}>
              <span className={styles.actionIcon}>👤</span>
              <strong>Edit Profile</strong>
              <p>Update your identity and contact info</p>
            </a>
            <a href="/vault" className={styles.actionCard}>
              <span className={styles.actionIcon}>📄</span>
              <strong>Upload Document</strong>
              <p>Scan a passport or ID to auto-fill</p>
            </a>
            <a href="/plugins" className={styles.actionCard}>
              <span className={styles.actionIcon}>🔌</span>
              <strong>Manage Plugins</strong>
              <p>Control which apps can access your data</p>
            </a>
            <a href="/settings" className={styles.actionCard}>
              <span className={styles.actionIcon}>⚙️</span>
              <strong>Settings</strong>
              <p>Security and preferences</p>
            </a>
          </div>
        </div>

        {profile && (
          <div className={styles.profilePreview}>
            <h3>Profile Summary</h3>
            <div className="card">
              {profile.identity?.full_name?.full_name && (
                <div className={styles.previewRow}>
                  <span>Name</span>
                  <strong>{profile.identity.full_name.full_name}</strong>
                </div>
              )}
              {profile.identity?.contact?.emails && profile.identity.contact.emails.length > 0 && (
                <div className={styles.previewRow}>
                  <span>Email</span>
                  <strong>{profile.identity.contact.emails[0].value}</strong>
                </div>
              )}
              {profile.travel?.primary_passport?.number && (
                <div className={styles.previewRow}>
                  <span>Passport</span>
                  <strong>{profile.travel.primary_passport.number}</strong>
                </div>
              )}
              {!profile.identity && (
                <p className={styles.emptyState}>
                  No profile data yet. Start by adding your information.
                </p>
              )}
            </div>
          </div>
        )}
      </div>
    </main>
  )
}

function calculateCompleteness(profile: Profile | null): number {
  if (!profile) return 0
  let filled = 0
  let total = 6

  if (profile.identity?.full_name?.full_name) filled++
  if (profile.identity?.date_of_birth) filled++
  if (profile.identity?.contact?.emails?.length) filled++
  if (profile.identity?.primary_address?.country) filled++
  if (profile.travel?.primary_passport?.number) filled++
  if (profile.professional?.employments?.length) filled++

  return Math.round((filled / total) * 100)
}
