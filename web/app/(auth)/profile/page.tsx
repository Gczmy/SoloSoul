'use client'

import { useState, useEffect, useRef } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { profileApi, Profile } from '@/lib/api'
import Header from '../Header'
import styles from './profile.module.css'

type Tab = 'identity' | 'travel' | 'financial' | 'professional' | 'preferences'

export default function ProfilePage() {
  const [activeTab, setActiveTab] = useState<Tab>('identity')
  const [profile, setProfile] = useState<Profile | null>(null)
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)
  const isSavingRef = useRef(false)
  const router = useRouter()
  const { dataVersion, bumpDataVersion, setProfileId } = useVaultStore()

  // Redirect to login if session is invalid
  useEffect(() => {
    const { sessionToken } = useVaultStore.getState()
    if (!sessionToken) {
      router.push('/login')
    }
  }, [router])

  // Load profile when dataVersion changes (after save) or on initial mount
  useEffect(() => {
    const loadProfile = async () => {
      // Skip if we're in the middle of saving - handleSave will handle the reload
      if (isSavingRef.current) return

      const { sessionToken } = useVaultStore.getState()
      if (!sessionToken) return

      try {
        // Always list profiles first to get the actual profile ID
        const listResult = await profileApi.list(sessionToken)
        if (!listResult.profile_ids || listResult.profile_ids.length === 0) return

        const actualProfileId = listResult.profile_ids[0]
        const result = await profileApi.get(actualProfileId, sessionToken)
        if (result.success && result.profile) {
          setProfile(result.profile)
        }
      } catch {
        // Failed to load profile
      }
    }

    loadProfile()
  }, [dataVersion])

  // Auto-clear message after 1 second
  useEffect(() => {
    if (message) {
      const timer = setTimeout(() => {
        setMessage(null)
      }, 1500)
      return () => clearTimeout(timer)
    }
  }, [message])

  const handleSave = async () => {
    const { sessionToken } = useVaultStore.getState()
    if (!profile || !sessionToken) return
    setSaving(true)
    setMessage(null)
    isSavingRef.current = true

    try {
      const result = await profileApi.update(profile, sessionToken)
      if (result.success) {
        setMessage({ type: 'success', text: 'Profile saved successfully' })
        // Update the profile ID if needed, but don't reload - the data we just saved is correct
        const listResult = await profileApi.list(sessionToken)
        if (listResult.profile_ids && listResult.profile_ids.length > 0) {
          setProfileId(listResult.profile_ids[0])
        }
      } else {
        setMessage({ type: 'error', text: result.error || 'Failed to save' })
      }
    } catch {
      setMessage({ type: 'error', text: 'Failed to save profile' })
    } finally {
      isSavingRef.current = false
      setSaving(false)
    }
  }

  const updateProfile = (path: string, value: unknown) => {
    // Initialize default profile structure if null
    let currentProfile = profile
    if (!currentProfile) {
      currentProfile = {
        profile_id: 'default',
        version: '1.0',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        identity: {
          full_name: { full_name: '' },
          contact: { emails: [], phones: [] },
        },
        travel: {},
        financial: {},
        professional: {},
        preferences: {},
      }
    }
    const newProfile = JSON.parse(JSON.stringify(currentProfile))

    // Ensure contact has arrays for emails and phones
    if (!newProfile.identity) {
      newProfile.identity = {}
    }
    if (!newProfile.identity.contact) {
      newProfile.identity.contact = { emails: [], phones: [] }
    }
    if (!newProfile.identity.contact.emails) {
      newProfile.identity.contact.emails = []
    }
    if (!newProfile.identity.contact.phones) {
      newProfile.identity.contact.phones = []
    }

    const parts = path.split('.')
    let obj: Record<string, unknown> = newProfile as Record<string, unknown>
    for (let i = 0; i < parts.length - 1; i++) {
      obj[parts[i]] = obj[parts[i]] || {}
      obj = obj[parts[i]] as Record<string, unknown>
    }
    obj[parts[parts.length - 1]] = value
    setProfile(newProfile)
  }

  return (
    <main className={styles.container}>
      <Header />
      <div className={styles.toolbar}>
        <h1>Profile</h1>
        <div className={styles.actions}>
          {message && (
            <span className={`${styles.message} ${styles[message.type]}`}>
              {message.text}
            </span>
          )}
          <button onClick={handleSave} className={`btn btn-primary ${styles.saveBtn}`} disabled={saving}>
            Save Changes
          </button>
        </div>
      </div>

      <div className={styles.tabs}>
        <button
          className={`tab ${activeTab === 'identity' ? 'active' : ''}`}
          onClick={() => setActiveTab('identity')}
        >
          Identity
        </button>
        <button
          className={`tab ${activeTab === 'travel' ? 'active' : ''}`}
          onClick={() => setActiveTab('travel')}
        >
          Travel
        </button>
        <button
          className={`tab ${activeTab === 'financial' ? 'active' : ''}`}
          onClick={() => setActiveTab('financial')}
        >
          Financial
        </button>
        <button
          className={`tab ${activeTab === 'professional' ? 'active' : ''}`}
          onClick={() => setActiveTab('professional')}
        >
          Professional
        </button>
        <button
          className={`tab ${activeTab === 'preferences' ? 'active' : ''}`}
          onClick={() => setActiveTab('preferences')}
        >
          Preferences
        </button>
      </div>

      <div className={styles.content}>
        {activeTab === 'identity' && (
          <IdentityTab profile={profile} updateProfile={updateProfile} />
        )}
        {activeTab === 'travel' && (
          <TravelTab profile={profile} updateProfile={updateProfile} />
        )}
        {activeTab === 'financial' && (
          <FinancialTab profile={profile} updateProfile={updateProfile} />
        )}
        {activeTab === 'professional' && (
          <ProfessionalTab profile={profile} updateProfile={updateProfile} />
        )}
        {activeTab === 'preferences' && (
          <PreferencesTab profile={profile} updateProfile={updateProfile} />
        )}
      </div>
    </main>
  )
}

function IdentityTab({
  profile,
  updateProfile,
}: {
  profile: Profile | null
  updateProfile: (path: string, value: unknown) => void
}) {
  const name = profile?.identity?.full_name || {}
  const contact = profile?.identity?.contact || { emails: [], phones: [] }
  const address = profile?.identity?.primary_address || {}
  const dob = profile?.identity?.date_of_birth

  const addEmail = () => {
    const emails = [...(contact.emails || []), { value: '', label: '' }]
    updateProfile('identity.contact.emails', emails)
  }

  const removeEmail = (index: number) => {
    const emails = (contact.emails || []).filter((_, i) => i !== index)
    updateProfile('identity.contact.emails', emails)
  }

  const updateEmail = (index: number, field: 'value' | 'label', val: string) => {
    const emails = [...(contact.emails || [])]
    emails[index] = { ...emails[index], [field]: val }
    updateProfile('identity.contact.emails', emails)
  }

  const addPhone = () => {
    const phones = [...(contact.phones || []), { value: '', label: '' }]
    updateProfile('identity.contact.phones', phones)
  }

  const removePhone = (index: number) => {
    const phones = (contact.phones || []).filter((_, i) => i !== index)
    updateProfile('identity.contact.phones', phones)
  }

  const updatePhone = (index: number, field: 'value' | 'label', val: string) => {
    const phones = [...(contact.phones || [])]
    phones[index] = { ...phones[index], [field]: val }
    updateProfile('identity.contact.phones', phones)
  }

  return (
    <div className={styles.form}>
      <section className={styles.section}>
        <h3>Name</h3>
        <div className={styles.grid}>
          <div className={styles.field}>
            <label className="label">Full Name</label>
            <input
              type="text"
              className="input"
              value={name.full_name || ''}
              onChange={(e) => updateProfile('identity.full_name.full_name', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Given Name</label>
            <input
              type="text"
              className="input"
              value={name.given_name || ''}
              onChange={(e) => updateProfile('identity.full_name.given_name', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Family Name</label>
            <input
              type="text"
              className="input"
              value={name.family_name || ''}
              onChange={(e) => updateProfile('identity.full_name.family_name', e.target.value)}
            />
          </div>
        </div>
      </section>

      <section className={styles.section}>
        <h3>Date of Birth</h3>
        <div className={styles.grid}>
          <div className={styles.field}>
            <label className="label">Year</label>
            <input
              type="number"
              className="input"
              value={dob?.year || ''}
              onChange={(e) => updateProfile('identity.date_of_birth.year', parseInt(e.target.value))}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Month</label>
            <input
              type="number"
              className="input"
              min="1"
              max="12"
              value={dob?.month || ''}
              onChange={(e) => updateProfile('identity.date_of_birth.month', parseInt(e.target.value))}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Day</label>
            <input
              type="number"
              className="input"
              min="1"
              max="31"
              value={dob?.day || ''}
              onChange={(e) => updateProfile('identity.date_of_birth.day', parseInt(e.target.value))}
            />
          </div>
        </div>
      </section>

      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <h3>Email</h3>
          <button type="button" onClick={addEmail} className={styles.addBtn}>+ Add</button>
        </div>
        {contact.emails && contact.emails.length > 0 ? (
          contact.emails.map((email, i) => (
            <div key={i} className={styles.contactRow}>
              <input
                type="text"
                className="input"
                placeholder="Label (e.g., Work, Personal)"
                value={email.label || ''}
                onChange={(e) => updateEmail(i, 'label', e.target.value)}
              />
              <input
                type="email"
                className="input"
                placeholder="Email address"
                value={email.value}
                onChange={(e) => updateEmail(i, 'value', e.target.value)}
              />
              <button type="button" onClick={() => removeEmail(i)} className={styles.removeBtn}>×</button>
            </div>
          ))
        ) : (
          <p className={styles.empty}>No email addresses added.</p>
        )}
      </section>

      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <h3>Phone</h3>
          <button type="button" onClick={addPhone} className={styles.addBtn}>+ Add</button>
        </div>
        {contact.phones && contact.phones.length > 0 ? (
          contact.phones.map((phone, i) => (
            <div key={i} className={styles.contactRow}>
              <input
                type="text"
                className="input"
                placeholder="Label (e.g., Work, Mobile)"
                value={phone.label || ''}
                onChange={(e) => updatePhone(i, 'label', e.target.value)}
              />
              <input
                type="tel"
                className="input"
                placeholder="Phone number"
                value={phone.value}
                onChange={(e) => updatePhone(i, 'value', e.target.value)}
              />
              <button type="button" onClick={() => removePhone(i)} className={styles.removeBtn}>×</button>
            </div>
          ))
        ) : (
          <p className={styles.empty}>No phone numbers added.</p>
        )}
      </section>

      <section className={styles.section}>
        <h3>Primary Address</h3>
        <div className={styles.field}>
          <label className="label">Street</label>
          <input
            type="text"
            className="input"
            value={address.street || ''}
            onChange={(e) => updateProfile('identity.primary_address.street', e.target.value)}
          />
        </div>
        <div className={styles.grid}>
          <div className={styles.field}>
            <label className="label">City</label>
            <input
              type="text"
              className="input"
              value={address.city || ''}
              onChange={(e) => updateProfile('identity.primary_address.city', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">State/Province</label>
            <input
              type="text"
              className="input"
              value={address.state || ''}
              onChange={(e) => updateProfile('identity.primary_address.state', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Postal Code</label>
            <input
              type="text"
              className="input"
              value={address.postal_code || ''}
              onChange={(e) => updateProfile('identity.primary_address.postal_code', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Country</label>
            <input
              type="text"
              className="input"
              value={address.country || ''}
              onChange={(e) => updateProfile('identity.primary_address.country', e.target.value)}
            />
          </div>
        </div>
      </section>
    </div>
  )
}

function TravelTab({
  profile,
  updateProfile,
}: {
  profile: Profile | null
  updateProfile: (path: string, value: unknown) => void
}) {
  const passport = profile?.travel?.primary_passport || {}

  return (
    <div className={styles.form}>
      <section className={styles.section}>
        <h3>Primary Passport</h3>
        <div className={styles.grid}>
          <div className={styles.field}>
            <label className="label">Passport Number</label>
            <input
              type="text"
              className="input"
              value={passport.number || ''}
              onChange={(e) => updateProfile('travel.primary_passport.number', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Country</label>
            <input
              type="text"
              className="input"
              value={passport.country || ''}
              onChange={(e) => updateProfile('travel.primary_passport.country', e.target.value)}
            />
          </div>
          <div className={styles.field}>
            <label className="label">Nationality</label>
            <input
              type="text"
              className="input"
              value={passport.nationality || ''}
              onChange={(e) => updateProfile('travel.primary_passport.nationality', e.target.value)}
            />
          </div>
        </div>
      </section>
    </div>
  )
}

function FinancialTab({
  profile,
  updateProfile,
}: {
  profile: Profile | null
  updateProfile: (path: string, value: unknown) => void
}) {
  const banks = profile?.financial?.bank_accounts || []

  return (
    <div className={styles.form}>
      <section className={styles.section}>
        <h3>Bank Accounts</h3>
        {banks.length === 0 ? (
          <p className={styles.empty}>No bank accounts added yet.</p>
        ) : (
          banks.map((bank, i) => (
            <div key={i} className={styles.card}>
              <div className={styles.field}>
                <label className="label">Bank Name</label>
                <input
                  type="text"
                  className="input"
                  value={bank.bank_name || ''}
                  onChange={(e) => updateProfile(`financial.bank_accounts.${i}.bank_name`, e.target.value)}
                />
              </div>
            </div>
          ))
        )}
      </section>
    </div>
  )
}

function ProfessionalTab({
  profile,
  updateProfile,
}: {
  profile: Profile | null
  updateProfile: (path: string, value: unknown) => void
}) {
  const employments = profile?.professional?.employments || []

  return (
    <div className={styles.form}>
      <section className={styles.section}>
        <h3>Employment</h3>
        {employments.length === 0 ? (
          <p className={styles.empty}>No employment history added yet.</p>
        ) : (
          employments.map((emp, i) => (
            <div key={i} className={styles.card}>
              <div className={styles.grid}>
                <div className={styles.field}>
                  <label className="label">Company</label>
                  <input
                    type="text"
                    className="input"
                    value={emp.company || ''}
                    onChange={(e) => updateProfile(`professional.employments.${i}.company`, e.target.value)}
                  />
                </div>
                <div className={styles.field}>
                  <label className="label">Title</label>
                  <input
                    type="text"
                    className="input"
                    value={emp.title || ''}
                    onChange={(e) => updateProfile(`professional.employments.${i}.title`, e.target.value)}
                  />
                </div>
              </div>
            </div>
          ))
        )}
      </section>
    </div>
  )
}

function PreferencesTab({
  profile,
  updateProfile,
}: {
  profile: Profile | null
  updateProfile: (path: string, value: unknown) => void
}) {
  const prefs = profile?.preferences || {}

  return (
    <div className={styles.form}>
      <section className={styles.section}>
        <h3>Travel Preferences</h3>
        <div className={styles.grid}>
          <div className={styles.field}>
            <label className="label">Meal Preference</label>
            <select
              className="input"
              value={prefs.meal_preference || ''}
              onChange={(e) => updateProfile('preferences.meal_preference', e.target.value)}
            >
              <option value="">Select...</option>
              <option value="regular">Regular</option>
              <option value="vegetarian">Vegetarian</option>
              <option value="vegan">Vegan</option>
              <option value="halal">Halal</option>
              <option value="kosher">Kosher</option>
              <option value="gluten_free">Gluten Free</option>
            </select>
          </div>
          <div className={styles.field}>
            <label className="label">Seat Preference</label>
            <select
              className="input"
              value={prefs.seat_preference || ''}
              onChange={(e) => updateProfile('preferences.seat_preference', e.target.value)}
            >
              <option value="">Select...</option>
              <option value="window">Window</option>
              <option value="aisle">Aisle</option>
              <option value="middle">Middle</option>
            </select>
          </div>
        </div>
      </section>
    </div>
  )
}
