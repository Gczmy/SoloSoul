package ocr

import (
	"context"
	"testing"
	"time"
)

func TestJobManager_NewJobManager(t *testing.T) {
	engine := &mockEngine{}
	m := NewJobManager(engine)

	if m == nil {
		t.Fatal("NewJobManager() returned nil")
	}

	if m.engine == nil {
		t.Error("engine should be set")
	}
	if m.jobs == nil {
		t.Error("jobs map should be initialized")
	}
}

func TestJobManager_GetJob_NotFound(t *testing.T) {
	engine := &mockEngine{}
	m := NewJobManager(engine)

	_, ok := m.GetJob("nonexistent-job")
	if ok {
		t.Error("GetJob() for nonexistent job should return ok=false")
	}
}

func TestJobManager_ListJobs_Empty(t *testing.T) {
	engine := &mockEngine{}
	m := NewJobManager(engine)

	jobs := m.ListJobs()
	if len(jobs) != 0 {
		t.Errorf("ListJobs() returned %d jobs, want 0", len(jobs))
	}
}

func TestJobManager_CleanupOldJobs(t *testing.T) {
	engine := &mockEngine{}
	m := NewJobManager(engine)

	// Manually add an old job
	oldJob := &OCRJob{
		ID:          "old-job",
		Status:      JobStatusCompleted,
		DocumentType: DocumentTypePassport,
		CreatedAt:   time.Now().Add(-2 * time.Hour),
	}

	m.mu.Lock()
	m.jobs[oldJob.ID] = oldJob
	m.mu.Unlock()

	// Cleanup jobs older than 1 hour
	m.CleanupOldJobs(1 * time.Hour)

	// Old job should be removed
	_, ok := m.GetJob("old-job")
	if ok {
		t.Error("Old job should have been cleaned up")
	}
}

func TestGenerateJobID(t *testing.T) {
	id1 := generateJobID()
	id2 := generateJobID()

	if id1 == "" {
		t.Error("generateJobID() returned empty string")
	}

	if id1 == id2 {
		t.Error("generateJobID() should produce unique IDs")
	}

	// Should start with ocr_
	if len(id1) < 4 || id1[:4] != "ocr_" {
		t.Errorf("generateJobID() = %q, should start with 'ocr_'", id1)
	}
}

func TestOCRJob_Fields(t *testing.T) {
	now := time.Now()
	job := &OCRJob{
		ID:          "job_123",
		Status:      JobStatusPending,
		DocumentType: DocumentTypePassport,
		CreatedAt:   now,
		CompletedAt: nil,
		Result:      nil,
		Error:       "",
	}

	if job.ID != "job_123" {
		t.Errorf("ID = %q, want %q", job.ID, "job_123")
	}
	if job.Status != JobStatusPending {
		t.Errorf("Status = %q, want %q", job.Status, JobStatusPending)
	}
	if !job.CreatedAt.Equal(now) {
		t.Errorf("CreatedAt = %v, want %v", job.CreatedAt, now)
	}
}

func TestJobStatusConstants(t *testing.T) {
	statuses := []JobStatus{
		JobStatusPending,
		JobStatusProcessing,
		JobStatusCompleted,
		JobStatusFailed,
	}

	// Verify they are distinct
	for i, s1 := range statuses {
		for j, s2 := range statuses {
			if i != j && s1 == s2 {
				t.Errorf("JobStatus constants at indices %d and %d should be distinct", i, j)
			}
		}
	}

	// Verify string values
	if JobStatusPending != "pending" {
		t.Errorf("JobStatusPending = %q, want %q", JobStatusPending, "pending")
	}
	if JobStatusProcessing != "processing" {
		t.Errorf("JobStatusProcessing = %q, want %q", JobStatusProcessing, "processing")
	}
	if JobStatusCompleted != "completed" {
		t.Errorf("JobStatusCompleted = %q, want %q", JobStatusCompleted, "completed")
	}
	if JobStatusFailed != "failed" {
		t.Errorf("JobStatusFailed = %q, want %q", JobStatusFailed, "failed")
	}
}

// mockEngine is a mock implementation of the Engine interface for testing
type mockEngine struct{}

func (m *mockEngine) ProcessImage(ctx context.Context, imageData []byte, docType DocumentType) (*ExtractionResult, error) {
	return &ExtractionResult{
		DocumentType: docType,
		Fields:      []ExtractedField{},
	}, nil
}

func (m *mockEngine) ProcessImageWithPreprocessing(ctx context.Context, imageData []byte, docType DocumentType, opts *PreprocessOptions) (*ExtractionResult, error) {
	return m.ProcessImage(ctx, imageData, docType)
}

func (m *mockEngine) DetectDocumentType(ctx context.Context, imageData []byte) (DocumentType, error) {
	return DocumentTypePassport, nil
}

func (m *mockEngine) Close() error {
	return nil
}
