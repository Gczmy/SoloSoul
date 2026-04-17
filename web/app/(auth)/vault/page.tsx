'use client'

import { useState } from 'react'
import Header from '../Header'
import styles from './vault.module.css'

export default function VaultPage() {
  const [documents] = useState<Array<{ id: string; type: string; title: string; date: string }>>([])

  return (
    <main className={styles.container}>
      <Header />
      <div className={styles.toolbar}>
        <h1>Vault</h1>
        <button className="btn btn-primary">Upload Document</button>
      </div>

      <div className={styles.content}>
        <div className={styles.filters}>
          <button className={`${styles.filter} ${styles.active}`}>All</button>
          <button className={styles.filter}>Passports</button>
          <button className={styles.filter}>IDs</button>
          <button className={styles.filter}>Visas</button>
          <button className={styles.filter}>Photos</button>
        </div>

        {documents.length === 0 ? (
          <div className={styles.empty}>
            <div className={styles.emptyIcon}>📄</div>
            <h3>No documents yet</h3>
            <p>Upload a passport or ID to auto-fill your profile</p>
            <button className="btn btn-primary">Upload Document</button>
          </div>
        ) : (
          <div className={styles.grid}>
            {documents.map((doc) => (
              <div key={doc.id} className={styles.card}>
                <div className={styles.cardIcon}>
                  {doc.type === 'passport' ? '🛂' : '📄'}
                </div>
                <div className={styles.cardContent}>
                  <strong>{doc.title}</strong>
                  <span>{doc.date}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </main>
  )
}
