'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useVaultStore } from '@/lib/store'
import { pluginApi, PluginManifest } from '@/lib/api'
import Header from '../Header'
import styles from './plugins.module.css'

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<Array<{ id: string; name: string; version: string; is_approved: boolean }>>([])
  const [loading, setLoading] = useState(true)
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null)
  const router = useRouter()
  const { sessionToken } = useVaultStore()

  useEffect(() => {
    // Redirect to login if session is invalid
    const { sessionToken } = useVaultStore.getState()
    if (!sessionToken) {
      router.push('/login')
      return
    }
    const loadPlugins = async () => {
      try {
        const data = await pluginApi.list(sessionToken)
        setPlugins(data.plugins || [])
      } catch (e) {
        console.error('Failed to load plugins', e)
        setPlugins([])
      } finally {
        setLoading(false)
      }
    }
    loadPlugins()
  }, [router])

  const handleApprove = async (pluginId: string) => {
    // TODO: Implement approval flow
    console.log('Approve plugin', pluginId)
  }

  return (
    <main className={styles.container}>
      <Header />

      <div className={styles.content}>
        <div className={styles.info}>
          <p>
            Plugins are external tools that can access your profile data with your permission.
            Only approve plugins from sources you trust.
          </p>
        </div>

        {loading ? (
          <div className={styles.loading}>Loading...</div>
        ) : !plugins || plugins.length === 0 ? (
          <div className={styles.empty}>
            <div className={styles.emptyIcon}>🔌</div>
            <h3>No plugins installed</h3>
            <p>Plugins will appear here when installed</p>
          </div>
        ) : (
          <div className={styles.list}>
            {plugins.map((plugin) => (
              <div key={plugin.id} className={styles.card}>
                <div className={styles.cardHeader}>
                  <div className={styles.pluginInfo}>
                    <strong>{plugin.name}</strong>
                    <span>v{plugin.version}</span>
                  </div>
                  <span className={`badge ${plugin.is_approved ? 'badge-success' : 'badge-warning'}`}>
                    {plugin.is_approved ? 'Approved' : 'Pending'}
                  </span>
                </div>
                <p className={styles.pluginId}>{plugin.id}</p>
                {!plugin.is_approved && (
                  <button
                    onClick={() => handleApprove(plugin.id)}
                    className="btn btn-primary"
                  >
                    Approve Access
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </main>
  )
}
