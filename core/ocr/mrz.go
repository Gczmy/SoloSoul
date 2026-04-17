package ocr

import (
	"fmt"
	"regexp"
	"strings"
	"time"
)

// MRZData represents parsed data from Machine Readable Zone
type MRZData struct {
	DocumentType    string `json:"document_type"`
	Country         string `json:"country"`
	Surname         string `json:"surname"`
	GivenNames      string `json:"given_names"`
	DocumentNumber  string `json:"document_number"`
	Nationality     string `json:"nationality"`
	DateOfBirth     string `json:"date_of_birth"`
	Sex             string `json:"sex"`
	ExpiryDate      string `json:"expiry_date"`
	PersonalNumber  string `json:"personal_number,omitempty"`
	CheckDigitDoc   string `json:"check_digit_doc,omitempty"`
	CheckDigitDOB   string `json:"check_digit_dob,omitempty"`
	CheckDigitExp   string `json:"check_digit_exp,omitempty"`
	CheckDigitPers  string `json:"check_digit_pers,omitempty"`
}

// MRZParser parses Machine Readable Zones from travel documents
type MRZParser struct{}

// NewMRZParser creates a new MRZ parser
func NewMRZParser() *MRZParser {
	return &MRZParser{}
}

// ParseTD3 parses a TD3 format MRZ (passport, 2 lines of 44 characters each)
func (p *MRZParser) ParseTD3(line1, line2 string) (*MRZData, error) {
	if len(line1) != 44 || len(line2) != 44 {
		return nil, fmt.Errorf("TD3 lines must be 44 characters each, got %d and %d", len(line1), len(line2))
	}

	// Remove any padding and whitespace
	line1 = strings.ToUpper(strings.TrimSpace(line1))
	line2 = strings.ToUpper(strings.TrimSpace(line2))

	data := &MRZData{}

	// Line 1: Type (2) + Country (3) + Names (39)
	data.DocumentType = line1[0:2]
	data.Country = line1[2:5]

	// Names are separated by <<
	nameParts := strings.Split(line1[5:], "<")
	if len(nameParts) >= 2 {
		data.Surname = strings.Trim(nameParts[0], "<")
		data.GivenNames = strings.Trim(strings.Join(nameParts[1:], " "), "<")
	}

	// Line 2: Document number + check digit + nationality + DOB + check digit + sex + expiry + check digit + personal number + check digit
	data.DocumentNumber = strings.Trim(line2[0:9], "<")
	data.CheckDigitDoc = line2[9:10]
	data.Nationality = line2[10:13]
	data.DateOfBirth = line2[13:19]
	data.CheckDigitDOB = line2[19:20]
	data.Sex = line2[20:21]
	data.ExpiryDate = line2[21:27]
	data.CheckDigitExp = line2[27:28]
	data.PersonalNumber = strings.Trim(line2[28:42], "<")
	data.CheckDigitPers = line2[42:43]

	// Validate check digits
	if !p.validateCheckDigit(data.DocumentNumber, data.CheckDigitDoc) {
		return nil, fmt.Errorf("document number check digit invalid")
	}
	if !p.validateCheckDigit(data.DateOfBirth, data.CheckDigitDOB) {
		return nil, fmt.Errorf("date of birth check digit invalid")
	}
	if !p.validateCheckDigit(data.ExpiryDate, data.CheckDigitExp) {
		return nil, fmt.Errorf("expiry date check digit invalid")
	}

	return data, nil
}

// ParseTD1 parses a TD1 format MRZ (ID cards, 3 lines of 30 characters each)
func (p *MRZParser) ParseTD1(line1, line2, line3 string) (*MRZData, error) {
	if len(line1) != 30 || len(line2) != 30 || len(line3) != 30 {
		return nil, fmt.Errorf("TD1 lines must be 30 characters each, got %d, %d, %d", len(line1), len(line2), len(line3))
	}

	line1 = strings.ToUpper(strings.TrimSpace(line1))
	line2 = strings.ToUpper(strings.TrimSpace(line2))
	line3 = strings.ToUpper(strings.TrimSpace(line3))

	data := &MRZData{}

	// Line 1: Type (2) + Country (3) + Names (26)
	data.DocumentType = line1[0:2]
	data.Country = line1[2:5]
	nameParts := strings.Split(line1[5:], "<")
	if len(nameParts) >= 2 {
		data.Surname = strings.Trim(nameParts[0], "<")
		data.GivenNames = strings.Trim(strings.Join(nameParts[1:], " "), "<")
	}

	// Line 2: Document number + check digit + DOB + check digit + sex + expiry + check digit
	data.DocumentNumber = strings.Trim(line2[0:9], "<")
	data.CheckDigitDoc = line2[9:10]
	data.DateOfBirth = line2[10:16]
	data.CheckDigitDOB = line2[16:17]
	data.Sex = line2[17:18]
	data.ExpiryDate = line2[18:24]
	data.CheckDigitExp = line2[24:25]

	// Line 3: Optional data
	data.PersonalNumber = strings.Trim(line3[0:14], "<")
	data.CheckDigitPers = line3[14:15]

	// Get nationality from country field (for ID cards)
	data.Nationality = data.Country

	return data, nil
}

// ParseTD2 parses a TD2 format MRZ (some IDs, 2 lines of 36 characters each)
func (p *MRZParser) ParseTD2(line1, line2 string) (*MRZData, error) {
	if len(line1) != 36 || len(line2) != 36 {
		return nil, fmt.Errorf("TD2 lines must be 36 characters each, got %d and %d", len(line1), len(line2))
	}

	line1 = strings.ToUpper(strings.TrimSpace(line1))
	line2 = strings.ToUpper(strings.TrimSpace(line2))

	data := &MRZData{}

	// Line 1: Type (2) + Country (3) + Names (31)
	data.DocumentType = line1[0:2]
	data.Country = line1[2:5]
	nameParts := strings.Split(line1[5:], "<")
	if len(nameParts) >= 2 {
		data.Surname = strings.Trim(nameParts[0], "<")
		data.GivenNames = strings.Trim(strings.Join(nameParts[1:], " "), "<")
	}

	// Line 2: Document number + check digit + nationality + DOB + check digit + sex + expiry + check digit + optional data
	data.DocumentNumber = strings.Trim(line2[0:9], "<")
	data.CheckDigitDoc = line2[9:10]
	data.Nationality = line2[10:13]
	data.DateOfBirth = line2[13:19]
	data.CheckDigitDOB = line2[19:20]
	data.Sex = line2[20:21]
	data.ExpiryDate = line2[21:27]
	data.CheckDigitExp = line2[27:28]
	data.PersonalNumber = strings.Trim(line2[28:35], "<")

	return data, nil
}

// CheckDigitWeights for MRZ validation
var mrzWeights = []int{7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1}

// validateCheckDigit validates an MRZ check digit
func (p *MRZParser) validateCheckDigit(data, checkDigit string) bool {
	if checkDigit == "<" {
		return true // No check digit for empty fields
	}

	sum := 0
	for i, c := range data {
		var val int
		switch {
		case c >= '0' && c <= '9':
			val = int(c - '0')
		case c >= 'A' && c <= 'Z':
			val = int(c - 'A' + 10)
		case c == '<':
			val = 0
		default:
			return false
		}
		sum += val * mrzWeights[i]
	}

	expected := sum % 10
	actual := int(checkDigit[0] - '0')
	return expected == actual
}

// ParseDate parses an MRZ date string (YYMMDD) to time.Time
func (p *MRZParser) ParseDate(dateStr string) (time.Time, error) {
	if len(dateStr) != 6 {
		return time.Time{}, fmt.Errorf("date must be 6 characters (YYMMDD)")
	}

	// MRZ dates areYYMMDD, need to determine century
	year := int(dateStr[0]-'0')*10 + int(dateStr[1]-'0')
	month := int(dateStr[2]-'0')*10 + int(dateStr[3]-'0')
	day := int(dateStr[4]-'0')*10 + int(dateStr[5]-'0')

	// Assume 1900s if year < 50, 2000s otherwise
	if year < 50 {
		year += 2000
	} else {
		year += 1900
	}

	return time.Date(year, time.Month(month), day, 0, 0, 0, 0, time.UTC), nil
}

// ToExtractedFields converts MRZData to ExtractedField slice
func (m *MRZData) ToExtractedFields() []ExtractedField {
	fields := []ExtractedField{
		{Key: "document_type", Value: m.DocumentType, Confidence: 0.99, Source: "mrz_type"},
		{Key: "country", Value: m.Country, Confidence: 0.99, Source: "mrz_country"},
		{Key: "surname", Value: m.Surname, Confidence: 0.98, Source: "mrz_name"},
		{Key: "given_names", Value: m.GivenNames, Confidence: 0.98, Source: "mrz_name"},
		{Key: "document_number", Value: m.DocumentNumber, Confidence: 0.99, Source: "mrz_docno"},
		{Key: "nationality", Value: m.Nationality, Confidence: 0.99, Source: "mrz_nationality"},
		{Key: "sex", Value: m.Sex, Confidence: 0.99, Source: "mrz_sex"},
	}

	if m.DateOfBirth != "<<<<<<" {
		fields = append(fields, ExtractedField{
			Key: "date_of_birth", Value: m.DateOfBirth, Confidence: 0.95, Source: "mrz_dob",
		})
	}

	if m.ExpiryDate != "<<<<<<" {
		fields = append(fields, ExtractedField{
			Key: "expiry_date", Value: m.ExpiryDate, Confidence: 0.95, Source: "mrz_expiry",
		})
	}

	if m.PersonalNumber != "" && m.PersonalNumber != "<<<<<<<<<<<<" {
		fields = append(fields, ExtractedField{
			Key: "personal_number", Value: m.PersonalNumber, Confidence: 0.90, Source: "mrz_personal",
		})
	}

	return fields
}

// ExtractMRZFromText attempts to find and parse MRZ lines from raw text
func (p *MRZParser) ExtractMRZFromText(text string) (*MRZData, error) {
	// TD3: Passport format (2 lines, 44 chars each)
	td3Regex := regexp.MustCompile(`([A-Z0-9<]{44})\s*([A-Z0-9<]{44})`)
	if matches := td3Regex.FindStringSubmatch(strings.ReplaceAll(text, "\n", "")); len(matches) == 3 {
		return p.ParseTD3(matches[1], matches[2])
	}

	// TD1: ID card format (3 lines, 30 chars each)
	td1Regex := regexp.MustCompile(`([A-Z0-9<]{30})\s*([A-Z0-9<]{30})\s*([A-Z0-9<]{30})`)
	if matches := td1Regex.FindStringSubmatch(strings.ReplaceAll(text, "\n", "")); len(matches) == 4 {
		return p.ParseTD1(matches[1], matches[2], matches[3])
	}

	// TD2: Some IDs (2 lines, 36 chars each)
	td2Regex := regexp.MustCompile(`([A-Z0-9<]{36})\s*([A-Z0-9<]{36})`)
	if matches := td2Regex.FindStringSubmatch(strings.ReplaceAll(text, "\n", "")); len(matches) == 3 {
		return p.ParseTD2(matches[1], matches[2])
	}

	return nil, fmt.Errorf("no valid MRZ found in text")
}
