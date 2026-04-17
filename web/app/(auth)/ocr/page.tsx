'use client'

import { useState, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import Header from '../Header'
import styles from './ocr.module.css'

type DocumentType = 'passport' | 'national_id' | 'visa' | 'driver_license'

interface ExtractedField {
  key: string
  value: string
  confidence: number
  source: string
}

const documentIcons: Record<DocumentType, string> = {
  passport: '🛂',
  national_id: '🪪',
  visa: '📋',
  driver_license: '🚗',
}

export default function OCRPage() {
  const [step, setStep] = useState<'upload' | 'preview' | 'confirm'>('upload')
  const [documentType, setDocumentType] = useState<DocumentType>('passport')
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [extractedFields, setExtractedFields] = useState<ExtractedField[]>([])
  const [editedFields, setEditedFields] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const router = useRouter()

  const handleFileSelect = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) return

    setSelectedFile(file)
    setError(null)

    const url = URL.createObjectURL(file)
    setPreviewUrl(url)
  }, [])

  const handleScan = async () => {
    if (!selectedFile) return

    setLoading(true)
    setError(null)

    try {
      const arrayBuffer = await selectedFile.arrayBuffer()
      const base64 = btoa(
        new Uint8Array(arrayBuffer).reduce((data, byte) => data + String.fromCharCode(byte), '')
      )

      await new Promise(resolve => setTimeout(resolve, 1500))

      const mockFields: ExtractedField[] = [
        { key: 'full_name', value: 'John Doe', confidence: 0.95, source: 'MRZ' },
        { key: 'passport_number', value: 'AB123456', confidence: 0.98, source: 'MRZ' },
        { key: 'country', value: 'United States', confidence: 0.92, source: 'MRZ' },
        { key: 'date_of_birth', value: '1990-01-15', confidence: 0.88, source: 'MRZ' },
        { key: 'expiry_date', value: '2030-01-14', confidence: 0.90, source: 'MRZ' },
      ]

      setExtractedFields(mockFields)
      const edited: Record<string, string> = {}
      mockFields.forEach(f => { edited[f.key] = f.value })
      setEditedFields(edited)
      setStep('preview')
    } catch {
      setError('Failed to scan document. Please try again.')
    }

    setLoading(false)
  }

  const handleFieldChange = (key: string, value: string) => {
    setEditedFields(prev => ({ ...prev, [key]: value }))
  }

  const handleConfirm = () => {
    setStep('confirm')
  }

  const handleSave = () => {
    router.push('/profile')
  }

  const handleBack = () => {
    setStep('upload')
    setExtractedFields([])
    setEditedFields({})
  }

  const handleReset = () => {
    setStep('upload')
    setSelectedFile(null)
    setPreviewUrl(null)
    setExtractedFields([])
    setEditedFields({})
    setError(null)
  }

  const getFieldLabel = (key: string) => {
    return key.split('_').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')
  }

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.9) return 'var(--color-success)'
    if (confidence >= 0.7) return 'var(--color-warning)'
    return 'var(--color-error)'
  }

  return (
    <main className={styles.container}>
      <Header />

      <div className={styles.content}>
        {step === 'upload' && (
          <div className={styles.uploadStep}>
            <div className={styles.stepHeader}>
              <h2>Scan Document</h2>
              <p>Upload a document to automatically extract and fill your profile data</p>
            </div>

            <div className={styles.typeGrid}>
              {(Object.keys(documentIcons) as DocumentType[]).map(type_ => (
                <button
                  key={type_}
                  className={`${styles.typeCard} ${documentType === type_ ? styles.active : ''}`}
                  onClick={() => setDocumentType(type_)}
                >
                  <span className={styles.typeIcon}>{documentIcons[type_]}</span>
                  <span className={styles.typeName}>{type_.replace('_', ' ')}</span>
                </button>
              ))}
            </div>

            <div className={styles.uploadZone}>
              <input
                type="file"
                accept="image/*"
                onChange={handleFileSelect}
                className={styles.fileInput}
                id="file-upload"
              />
              <label htmlFor="file-upload" className={styles.uploadLabel}>
                {previewUrl ? (
                  <img src={previewUrl} alt="Preview" className={styles.preview} />
                ) : (
                  <>
                    <span className={styles.uploadIcon}>📷</span>
                    <span className={styles.uploadText}>
                      {selectedFile ? selectedFile.name : 'Click to select or drag image here'}
                    </span>
                  </>
                )}
              </label>
              {previewUrl && (
                <button className={styles.clearButton} onClick={() => {
                  setSelectedFile(null)
                  setPreviewUrl(null)
                }}>
                  ✕ Clear
                </button>
              )}
            </div>

            {error && <div className={styles.error}>{error}</div>}

            <div className={styles.actions}>
              <button
                className="btn btn-primary"
                onClick={handleScan}
                disabled={!selectedFile || loading}
              >
                {loading ? (
                  <>
                    <span className={styles.spinner} />
                    Scanning...
                  </>
                ) : (
                  <>Scan Document</>
                )}
              </button>
            </div>
          </div>
        )}

        {step === 'preview' && (
          <div className={styles.previewStep}>
            <div className={styles.stepHeader}>
              <h2>Verify Data</h2>
              <p>Review the extracted fields and make corrections if needed</p>
            </div>

            <div className={styles.previewLayout}>
              <div className={styles.imageSection}>
                {previewUrl && <img src={previewUrl} alt="Document" className={styles.docImage} />}
              </div>

              <div className={styles.fieldsSection}>
                {extractedFields.map(field => (
                  <div key={field.key} className={styles.fieldCard}>
                    <div className={styles.fieldHeader}>
                      <label className={styles.fieldLabel}>{getFieldLabel(field.key)}</label>
                      <span
                        className={styles.confidence}
                        style={{ color: getConfidenceColor(field.confidence) }}
                      >
                        {Math.round(field.confidence * 100)}%
                      </span>
                    </div>
                    <input
                      type="text"
                      className="input"
                      value={editedFields[field.key] || ''}
                      onChange={e => handleFieldChange(field.key, e.target.value)}
                    />
                  </div>
                ))}
              </div>
            </div>

            <div className={styles.actions}>
              <button className="btn btn-secondary" onClick={handleBack}>
                ← Back
              </button>
              <button className="btn btn-primary" onClick={handleConfirm}>
                Confirm & Save
              </button>
            </div>
          </div>
        )}

        {step === 'confirm' && (
          <div className={styles.confirmStep}>
            <div className={styles.successIcon}>✓</div>
            <h2>Document Saved</h2>
            <p>
              Your {documentType.replace('_', ' ')} has been scanned and the data has been added to your profile.
            </p>
            <div className={styles.actions}>
              <button className="btn btn-secondary" onClick={handleReset}>
                Scan Another
              </button>
              <button className="btn btn-primary" onClick={handleSave}>
                View Profile
              </button>
            </div>
          </div>
        )}
      </div>
    </main>
  )
}
