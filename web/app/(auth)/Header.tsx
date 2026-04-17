'use client'

import { useState } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { LockIcon, HomeIcon, UserIcon, VaultIcon, ScanIcon, PlugIcon, SettingsIcon } from '@/components/Icons'
import styles from './Header.module.css'

interface NavItem {
  href: string
  label: string
  icon: React.ReactNode
}

const navItems: NavItem[] = [
  { href: '/dashboard', label: 'Dashboard', icon: <HomeIcon size={18} /> },
  { href: '/profile', label: 'Profile', icon: <UserIcon size={18} /> },
  { href: '/vault', label: 'Vault', icon: <VaultIcon size={18} /> },
  { href: '/ocr', label: 'Scan', icon: <ScanIcon size={18} /> },
  { href: '/plugins', label: 'Plugins', icon: <PlugIcon size={18} /> },
  { href: '/settings', label: 'Settings', icon: <SettingsIcon size={18} /> },
]

export default function Header() {
  const pathname = usePathname()
  const router = useRouter()
  const { lock, currentAccount, accounts, _hasHydrated } = useVaultStore()
  const [showLockModal, setShowLockModal] = useState(false)

  const handleLockClick = () => {
    setShowLockModal(true)
  }

  const handleConfirmLock = async () => {
    setShowLockModal(false)
    await lock()
    router.push('/login')
  }

  const handleCancelLock = () => {
    setShowLockModal(false)
  }

  const currentAccountName =
    accounts.find((a) => a.id === currentAccount)?.name ||
    (currentAccount ? currentAccount.slice(0, 8) : null)

  return (
    <>
      <aside className={styles.sidebar}>
        <Link href="/home" className={styles.logo}>
          SoloSoul
        </Link>

        <div className={`${!_hasHydrated ? styles.hydrating : ''}`}>
          {currentAccountName && (
            <div className={styles.accountBadge}>
              <span className={styles.accountIcon}>
                {currentAccountName.charAt(0).toUpperCase()}
              </span>
              <span className={styles.accountName}>{currentAccountName}</span>
            </div>
          )}

          <nav className={styles.nav}>
            {navItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className={`${styles.navLink} ${item.href === pathname ? styles.active : ''}`}
              >
                <span className={styles.navIcon}>{item.icon}</span>
                {item.label}
              </Link>
            ))}
          </nav>
          <div className={styles.footer}>
            <button onClick={handleLockClick} className={styles.lockButton}>
              <span className={styles.lockIcon}><LockIcon size={18} /></span>
              Lock Vault
            </button>
          </div>
        </div>
      </aside>

      {/* Lock Confirmation Modal */}
      {showLockModal && (
        <div className={styles.modalOverlay} onClick={handleCancelLock}>
          <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
            <div className={styles.modalIcon}>
              <LockIcon size={28} />
            </div>
            <h3 className={styles.modalTitle}>Lock Vault</h3>
            <p className={styles.modalText}>
              Your vault will be locked. You will need to enter your master password to unlock it again.
            </p>
            <div className={styles.modalActions}>
              <button onClick={handleCancelLock} className={styles.modalCancel}>
                Cancel
              </button>
              <button onClick={handleConfirmLock} className={styles.modalConfirm}>
                Lock Vault
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
