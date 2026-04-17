package ocr

import (
	"testing"
	"time"
)

func TestMRZParser_ParseTD3(t *testing.T) {
	p := NewMRZParser()

	// Valid TD3: 44 chars each line (pre-validated)
	line1Valid := "P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	line2Valid := "0123456789GBR8101011F2507144<<<<<<<<<<<<<<4<"

	tests := []struct {
		name      string
		line1     string
		line2     string
		wantErr   bool
		checkFunc func(*MRZData) bool
	}{
		{
			name:    "valid TD3",
			line1:   line1Valid,
			line2:   line2Valid,
			wantErr: false,
			checkFunc: func(m *MRZData) bool {
				return m.DocumentType == "P<" &&
					m.Country == "GBR" &&
					m.Surname == "SMITH" &&
					m.GivenNames == "SARAH" &&
					m.DocumentNumber == "012345678" &&
					m.Nationality == "GBR" &&
					m.DateOfBirth == "810101" &&
					m.Sex == "F" &&
					m.ExpiryDate == "250714"
			},
		},
		{
			name:    "invalid line length line1",
			line1:   "P<GBRSMITH<<SARAH",
			line2:   line2Valid,
			wantErr: true,
		},
		{
			name:    "invalid line length line2",
			line1:   line1Valid,
			line2:   "0123456789",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data, err := p.ParseTD3(tt.line1, tt.line2)
			if tt.wantErr {
				if err == nil {
					t.Error("ParseTD3() expected error, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("ParseTD3() unexpected error: %v", err)
			}
			if tt.checkFunc != nil && !tt.checkFunc(data) {
				t.Errorf("ParseTD3() data = %+v, check failed", data)
			}
		})
	}
}

func TestMRZParser_ParseTD1(t *testing.T) {
	p := NewMRZParser()

	// Valid TD1: 30 chars each line
	line1Valid := "I<GBRSMITH<<SARAH<<<<<<<<<<<<<<"  // 30 chars
	line2Valid := "0123456789<4GBR8101011F250714<<" // 30 chars
	line3Valid := "<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"   // 30 chars

	tests := []struct {
		name      string
		line1     string
		line2     string
		line3     string
		wantErr   bool
		checkFunc func(*MRZData) bool
	}{
		{
			name:    "valid TD1",
			line1:   line1Valid,
			line2:   line2Valid,
			line3:   line3Valid,
			wantErr: false,
			checkFunc: func(m *MRZData) bool {
				return m.DocumentType == "I<" &&
					m.Country == "GBR" &&
					m.Surname == "SMITH" &&
					m.GivenNames == "SARAH" &&
					m.DocumentNumber == "0123456789" &&
					m.DateOfBirth == "810101" &&
					m.Sex == "F" &&
					m.ExpiryDate == "250714"
			},
		},
		{
			name:    "invalid line1 length",
			line1:   "I<GBR",
			line2:   line2Valid,
			line3:   line3Valid,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data, err := p.ParseTD1(tt.line1, tt.line2, tt.line3)
			if tt.wantErr {
				if err == nil {
					t.Error("ParseTD1() expected error, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("ParseTD1() unexpected error: %v", err)
			}
			if tt.checkFunc != nil && !tt.checkFunc(data) {
				t.Errorf("ParseTD1() data = %+v, check failed", data)
			}
		})
	}
}

func TestMRZParser_ParseTD2(t *testing.T) {
	p := NewMRZParser()

	// Valid TD2: 36 chars each line
	line1Valid := "I<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<" // 36 chars
	line2Valid := "012345678<4GBR8101011F250714<<<<<"  // 36 chars

	tests := []struct {
		name      string
		line1     string
		line2     string
		wantErr   bool
		checkFunc func(*MRZData) bool
	}{
		{
			name:    "valid TD2",
			line1:   line1Valid,
			line2:   line2Valid,
			wantErr: false,
			checkFunc: func(m *MRZData) bool {
				return m.DocumentType == "I<" &&
					m.Country == "GBR" &&
					m.Surname == "SMITH" &&
					m.GivenNames == "SARAH" &&
					m.DocumentNumber == "012345678" &&
					m.Nationality == "GBR" &&
					m.DateOfBirth == "810101" &&
					m.Sex == "F" &&
					m.ExpiryDate == "250714"
			},
		},
		{
			name:    "invalid line1 length",
			line1:   "I<GBR",
			line2:   line2Valid,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data, err := p.ParseTD2(tt.line1, tt.line2)
			if tt.wantErr {
				if err == nil {
					t.Error("ParseTD2() expected error, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("ParseTD2() unexpected error: %v", err)
			}
			if tt.checkFunc != nil && !tt.checkFunc(data) {
				t.Errorf("ParseTD2() data = %+v, check failed", data)
			}
		})
	}
}

func TestMRZParser_ValidateCheckDigit(t *testing.T) {
	p := &MRZParser{}

	// Test with known valid and invalid check digits
	// MRZ weights: 7, 3, 1 repeated
	// For "12345678": 7*1 + 3*2 + 1*3 + 7*4 + 3*5 + 1*6 + 7*7 + 3*8 = 138, 138%10 = 8

	tests := []struct {
		name        string
		data        string
		checkDigit  string
		wantValid   bool
	}{
		{"valid number 12345678", "12345678", "8", true},
		{"invalid check digit", "12345678", "5", false},
		{"valid with < filler", "<<<<<<", "<", true},
		{"empty field accepts <", "ABC", "<", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := p.validateCheckDigit(tt.data, tt.checkDigit)
			if got != tt.wantValid {
				t.Errorf("validateCheckDigit(%q, %q) = %v, want %v", tt.data, tt.checkDigit, got, tt.wantValid)
			}
		})
	}
}

func TestMRZParser_ParseDate(t *testing.T) {
	p := NewMRZParser()

	// Implementation: year < 50 ? year + 2000 : year + 1900
	tests := []struct {
		name      string
		dateStr   string
		wantErr   bool
		checkFunc func(time.Time) bool
	}{
		{
			name:    "year 90 -> 1990",
			dateStr: "900101",
			wantErr: false,
			checkFunc: func(t time.Time) bool {
				return t.Year() == 1990 && t.Month() == 1 && t.Day() == 1
			},
		},
		{
			name:    "year 00 -> 2000",
			dateStr: "000101",
			wantErr: false,
			checkFunc: func(t time.Time) bool {
				return t.Year() == 2000 && t.Month() == 1 && t.Day() == 1
			},
		},
		{
			name:    "year 49 -> 1949",
			dateStr: "490101",
			wantErr: false,
			checkFunc: func(t time.Time) bool {
				return t.Year() == 1949 && t.Month() == 1 && t.Day() == 1
			},
		},
		{
			name:    "invalid length",
			dateStr: "9001",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := p.ParseDate(tt.dateStr)
			if tt.wantErr {
				if err == nil {
					t.Error("ParseDate() expected error, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("ParseDate() unexpected error: %v", err)
			}
			if tt.checkFunc != nil && !tt.checkFunc(got) {
				t.Errorf("ParseDate(%q) = %v, check failed", tt.dateStr, got)
			}
		})
	}
}

func TestMRZParser_ExtractMRZFromText(t *testing.T) {
	p := NewMRZParser()

	// Valid TD3 strings
	line1 := "P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	line2 := "0123456789GBR8101011F2507144<<<<<<<<<<<<<<4<"

	tests := []struct {
		name      string
		text      string
		wantErr   bool
		checkFunc func(*MRZData) bool
	}{
		{
			name: "TD3 in continuous text",
			text: "Some text before " + line1 + line2 + " more text after",
			wantErr: false,
			checkFunc: func(m *MRZData) bool {
				return m.DocumentType == "P<" && m.Surname == "SMITH"
			},
		},
		{
			name: "TD3 with newlines",
			text: line1 + "\n" + line2,
			wantErr: false,
			checkFunc: func(m *MRZData) bool {
				return m.DocumentType == "P<" && m.Surname == "SMITH"
			},
		},
		{
			name:    "no valid MRZ",
			text:    "This is just some random text without any MRZ data",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data, err := p.ExtractMRZFromText(tt.text)
			if tt.wantErr {
				if err == nil {
					t.Error("ExtractMRZFromText() expected error, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("ExtractMRZFromText() unexpected error: %v", err)
			}
			if tt.checkFunc != nil && !tt.checkFunc(data) {
				t.Errorf("ExtractMRZFromText() data = %+v, check failed", data)
			}
		})
	}
}

func TestMRZData_ToExtractedFields(t *testing.T) {
	data := &MRZData{
		DocumentType:   "P<",
		Country:        "USA",
		Surname:        "SMITH",
		GivenNames:     "JOHN",
		DocumentNumber: "P12345678",
		Nationality:    "USA",
		DateOfBirth:    "900101",
		Sex:            "M",
		ExpiryDate:     "300123",
		PersonalNumber: "1234567890",
	}

	fields := data.ToExtractedFields()

	// Should have base fields plus optional fields
	if len(fields) < 7 {
		t.Errorf("ToExtractedFields() returned %d fields, want at least 7", len(fields))
	}

	// Check that key fields exist
	fieldMap := make(map[string]string)
	for _, f := range fields {
		fieldMap[f.Key] = f.Value
	}

	expectedFields := []string{"document_type", "country", "surname", "given_names", "document_number", "nationality", "sex", "date_of_birth", "expiry_date", "personal_number"}
	for _, key := range expectedFields {
		if _, ok := fieldMap[key]; !ok {
			t.Errorf("ToExtractedFields() missing field: %s", key)
		}
	}
}

func TestMRZData_ToExtractedFields_WithEmptyOptional(t *testing.T) {
	data := &MRZData{
		DocumentType:   "P<",
		Country:        "USA",
		Surname:        "SMITH",
		GivenNames:     "JOHN",
		DocumentNumber: "P12345678",
		Nationality:    "USA",
		DateOfBirth:    "<<<<<<", // Empty in MRZ
		Sex:            "M",
		ExpiryDate:     "<<<<<<", // Empty in MRZ
		PersonalNumber: "<<<<<<<<<<<<", // Empty in MRZ
	}

	fields := data.ToExtractedFields()

	// Should NOT include empty optional fields
	fieldMap := make(map[string]string)
	for _, f := range fields {
		fieldMap[f.Key] = f.Value
	}

	if _, ok := fieldMap["date_of_birth"]; ok {
		t.Error("ToExtractedFields() should not include empty date_of_birth")
	}
	if _, ok := fieldMap["expiry_date"]; ok {
		t.Error("ToExtractedFields() should not include empty expiry_date")
	}
	if _, ok := fieldMap["personal_number"]; ok {
		t.Error("ToExtractedFields() should not include empty personal_number")
	}
}

func TestMRZParser_NameParsing(t *testing.T) {
	p := NewMRZParser()

	line2 := "0123456789GBR8101011F2507144<<<<<<<<<<<<<<4<"

	tests := []struct {
		name        string
		line1       string
		wantSurname string
		wantGiven   string
	}{
		{
			name:        "simple name",
			line1:       "P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<",
			wantSurname: "SMITH",
			wantGiven:   "SARAH",
		},
		{
			name:        "name with multiple given names",
			line1:       "P<GBRSMITH<<SARAH<ELIZABETH<<<<<<<<<<<<<<<<<<",
			wantSurname: "SMITH",
			wantGiven:   "SARAH ELIZABETH",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data, err := p.ParseTD3(tt.line1, line2)
			if err != nil {
				t.Fatalf("ParseTD3() failed: %v", err)
			}
			if data.Surname != tt.wantSurname {
				t.Errorf("Surname = %q, want %q", data.Surname, tt.wantSurname)
			}
			if data.GivenNames != tt.wantGiven {
				t.Errorf("GivenNames = %q, want %q", data.GivenNames, tt.wantGiven)
			}
		})
	}
}
