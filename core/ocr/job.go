package ocr

import (
	"context"
	"fmt"
	"sync"
	"time"
)

// JobManager manages OCR jobs
type JobManager struct {
	engine Engine
	jobs   map[string]*OCRJob
	mu     sync.RWMutex
}

// NewJobManager creates a new job manager
func NewJobManager(engine Engine) *JobManager {
	return &JobManager{
		engine: engine,
		jobs:   make(map[string]*OCRJob),
	}
}

// SubmitJob submits a new OCR job
func (m *JobManager) SubmitJob(imageData []byte, docType DocumentType) (*OCRJob, error) {
	job := &OCRJob{
		ID:          generateJobID(),
		Status:      JobStatusPending,
		DocumentType: docType,
		CreatedAt:   time.Now(),
	}

	m.mu.Lock()
	m.jobs[job.ID] = job
	m.mu.Unlock()

	// Process asynchronously
	go m.processJob(job.ID, imageData, docType)

	return job, nil
}

// GetJob returns a job by ID
func (m *JobManager) GetJob(id string) (*OCRJob, bool) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	job, ok := m.jobs[id]
	return job, ok
}

// processJob processes the OCR job
func (m *JobManager) processJob(id string, imageData []byte, docType DocumentType) {
	m.mu.Lock()
	job := m.jobs[id]
	job.Status = JobStatusProcessing
	m.mu.Unlock()

	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	result, err := m.engine.ProcessImage(ctx, imageData, docType)

	m.mu.Lock()
	defer m.mu.Unlock()

	job = m.jobs[id]
	job.Status = JobStatusCompleted
	now := time.Now()
	job.CompletedAt = &now

	if err != nil {
		job.Status = JobStatusFailed
		job.Error = err.Error()
	} else {
		job.Result = result
	}
}

// ListJobs returns all jobs
func (m *JobManager) ListJobs() []*OCRJob {
	m.mu.RLock()
	defer m.mu.RUnlock()

	jobs := make([]*OCRJob, 0, len(m.jobs))
	for _, job := range m.jobs {
		jobs = append(jobs, job)
	}
	return jobs
}

// CleanupOldJobs removes jobs older than the given duration
func (m *JobManager) CleanupOldJobs(olderThan time.Duration) {
	m.mu.Lock()
	defer m.mu.Unlock()

	cutoff := time.Now().Add(-olderThan)
	for id, job := range m.jobs {
		if job.CreatedAt.Before(cutoff) {
			delete(m.jobs, id)
		}
	}
}

func generateJobID() string {
	return fmt.Sprintf("ocr_%d", time.Now().UnixNano())
}
