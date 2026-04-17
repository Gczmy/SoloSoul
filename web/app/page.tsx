'use client'

import { useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'

export default function HomePage() {
  const router = useRouter()
  const { checkStatus } = useVaultStore()

  useEffect(() => {
    checkStatus().then(() => {
      // Use getState() to get the updated values after checkStatus completes
      const { isInitialized, isLocked } = useVaultStore.getState()
      if (!isInitialized) {
        router.push('/setup')
      } else if (isLocked) {
        router.push('/login')
      } else {
        router.push('/dashboard')
      }
    })
  }, [router, checkStatus])

  return (
    <div style={{ minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <p>Loading...</p>
    </div>
  )
}
