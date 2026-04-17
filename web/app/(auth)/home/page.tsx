'use client'

import { useState, useEffect, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import Header from '../Header'
import styles from './home.module.css'

const visions = [
  {
    id: 'security',
    title: 'Zero-Knowledge Architecture',
    subtitle: 'Local Encryption · Zero Exposure',
    description: 'Your data encrypted with Argon2id + AES-256-GCM. The server never touches your master password or plaintext data.',
    icon: 'lock',
  },
  {
    id: 'identity',
    title: 'Unified Digital Identity',
    subtitle: 'One Schema · Infinite Storage',
    description: 'Passports, financial accounts, professional history — structured once, stored wherever you choose.',
    icon: 'user',
  },
  {
    id: 'agent',
    title: 'AI Agent Interface',
    subtitle: 'Reason Locally · Act Precisely',
    description: 'Grant temporary, revocable access to AI agents. Let them reason about your data without exposing it.',
    icon: 'ai',
  },
  {
    id: 'local',
    title: 'Local-First Philosophy',
    subtitle: 'Your Device · Your Data',
    description: 'No cloud dependency. No vendor lock-in. Your digital twin runs on your hardware, under your control.',
    icon: 'server',
  },
]

export default function HomePage() {
  const router = useRouter()
  const { isLocked, _hasHydrated } = useVaultStore()
  const [activeIndex, setActiveIndex] = useState(0)
  const [isTransitioning, setIsTransitioning] = useState(false)

  const cycleVision = useCallback(() => {
    setIsTransitioning(true)
    setTimeout(() => {
      setActiveIndex((prev) => (prev + 1) % visions.length)
      setIsTransitioning(false)
    }, 400)
  }, [])

  useEffect(() => {
    // Wait for store to rehydrate from localStorage before checking lock state
    if (!_hasHydrated) return
    if (isLocked) {
      router.push('/login')
      return
    }
    const interval = setInterval(cycleVision, 5000)
    return () => clearInterval(interval)
  }, [_hasHydrated, isLocked, router, cycleVision])

  const activeVision = visions[activeIndex]

  return (
    <main className={styles.container}>
      <Header />
      <div className={styles.canvas}>
        <div className={styles.gridOverlay} />
        <div className={styles.orb} />
        <div className={styles.orb2} />
        <div className={styles.orb3} />
      </div>

      <div className={styles.content}>
        <div className={styles.hero}>
          <div className={styles.logoMark}>
            <svg viewBox="0 0 60 60" className={styles.logoSvg}>
              <defs>
                <linearGradient id="logoGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="#60a5fa" />
                  <stop offset="50%" stopColor="#3b82f6" />
                  <stop offset="100%" stopColor="#2563eb" />
                </linearGradient>
              </defs>
              <circle cx="30" cy="30" r="28" fill="none" stroke="url(#logoGrad)" strokeWidth="2" />
              <circle cx="30" cy="30" r="20" fill="none" stroke="url(#logoGrad)" strokeWidth="1.5" opacity="0.6" />
              <circle cx="30" cy="30" r="12" fill="none" stroke="url(#logoGrad)" strokeWidth="1" opacity="0.4" />
              <circle cx="30" cy="30" r="4" fill="url(#logoGrad)" />
            </svg>
          </div>
          <h1 className={styles.title}>SoloSoul</h1>
          <p className={styles.subtitle}>Digital Twin Engine</p>
        </div>

        <div className={`${styles.visionCard} ${isTransitioning ? styles.fadeOut : styles.fadeIn}`}>
          <div className={styles.visionIcon}>
            <VisionIcon type={activeVision.icon} />
          </div>
          <div className={styles.visionContent}>
            <span className={styles.visionSubtitle}>{activeVision.subtitle}</span>
            <h2 className={styles.visionTitle}>{activeVision.title}</h2>
            <p className={styles.visionDesc}>{activeVision.description}</p>
          </div>
        </div>

        <div className={styles.dots}>
          {visions.map((_, i) => (
            <button
              key={i}
              className={`${styles.dot} ${i === activeIndex ? styles.dotActive : ''}`}
              onClick={() => {
                setIsTransitioning(true)
                setTimeout(() => {
                  setActiveIndex(i)
                  setIsTransitioning(false)
                }, 400)
              }}
            />
          ))}
        </div>

        <div className={styles.cta}>
          <button onClick={() => router.push('/dashboard')} className={styles.ctaButton}>
            <span>Enter Dashboard</span>
            <svg viewBox="0 0 24 24" className={styles.arrowIcon}>
              <path d="M5 12h14M12 5l7 7-7 7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
        </div>

        <div className={styles.quote}>
          <p>&ldquo;Centralized Schema definition, decentralized data storage&rdquo;</p>
        </div>
      </div>
    </main>
  )
}

function VisionIcon({ type }: { type: string }) {
  switch (type) {
    case 'lock':
      return (
        <svg viewBox="0 0 48 48" className={styles.iconSvg}>
          <rect x="10" y="22" width="28" height="22" rx="3" fill="none" stroke="currentColor" strokeWidth="2" />
          <path d="M16 22v-8a8 8 0 1116 0v8" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          <circle cx="24" cy="33" r="3" fill="currentColor" />
          <line x1="24" y1="36" x2="24" y2="39" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      )
    case 'user':
      return (
        <svg viewBox="0 0 48 48" className={styles.iconSvg}>
          <circle cx="24" cy="16" r="8" fill="none" stroke="currentColor" strokeWidth="2" />
          <path d="M8 42c0-8.837 7.163-16 16-16s16 7.163 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      )
    case 'ai':
      return (
        <svg viewBox="0 0 48 48" className={styles.iconSvg}>
          <circle cx="24" cy="24" r="16" fill="none" stroke="currentColor" strokeWidth="2" />
          <circle cx="24" cy="24" r="8" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <circle cx="24" cy="24" r="3" fill="currentColor" />
          <line x1="24" y1="2" x2="24" y2="10" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          <line x1="24" y1="38" x2="24" y2="46" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          <line x1="2" y1="24" x2="10" y2="24" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          <line x1="38" y1="24" x2="46" y2="24" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      )
    case 'server':
      return (
        <svg viewBox="0 0 48 48" className={styles.iconSvg}>
          <rect x="6" y="8" width="36" height="12" rx="2" fill="none" stroke="currentColor" strokeWidth="2" />
          <rect x="6" y="24" width="36" height="12" rx="2" fill="none" stroke="currentColor" strokeWidth="2" />
          <circle cx="14" cy="14" r="2" fill="currentColor" />
          <circle cx="14" cy="30" r="2" fill="currentColor" />
          <line x1="22" y1="14" x2="36" y2="14" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          <line x1="22" y1="30" x2="36" y2="30" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      )
    default:
      return null
  }
}
