package schema

import (
	"time"
)

// FieldType represents the type of a schema field
type FieldType int

const (
	FieldTypeString FieldType = iota
	FieldTypeInt
	FieldTypeBool
	FieldTypeDate
	FieldTypeTimestamp
	FieldTypeBinary
	FieldTypeEnum
	FieldTypeCompound
	FieldTypeList
	FieldTypeSensitive
)

// EncryptionLevel determines how a field is encrypted
type EncryptionLevel int

const (
	EncryptAtRest EncryptionLevel = iota // Default
	EncryptAlways                         // For highly sensitive (SSN, etc)
	EncryptNever                          // Public data only
)

// SuperProfile is the complete user profile
type SuperProfile struct {
	Version     string            `json:"version"`
	ProfileID   string            `json:"profile_id"`
	CreatedAt   time.Time         `json:"created_at"`
	UpdatedAt   time.Time         `json:"updated_at"`
	Identity    Identity          `json:"identity"`
	Travel      Travel            `json:"travel"`
	Financial   Financial         `json:"financial"`
	Professional Professional     `json:"professional"`
	Preferences Preferences       `json:"preferences"`
	Documents   []DocumentRef     `json:"documents,omitempty"`
	Metadata    ProfileMetadata   `json:"metadata"`
}

// Identity contains core identity information
type Identity struct {
	FullName         PersonName       `json:"full_name"`
	Passport         *Passport        `json:"passport,omitempty"`
	NationalID       *NationalID      `json:"national_id,omitempty"`
	DateOfBirth      Date             `json:"date_of_birth"`
	Gender           Gender           `json:"gender"`
	BirthPlace       string           `json:"birth_place"`
	Photo            *Photo           `json:"photo,omitempty"`
	Signature        *Photo           `json:"signature,omitempty"`
	Contact          Contact          `json:"contact"`
	PrimaryAddress   Address          `json:"primary_address"`
	SecondaryAddress *Address         `json:"secondary_address,omitempty"`
	EmergencyContact *EmergencyContact `json:"emergency_contact,omitempty"`
}

// PersonName represents a person's name
type PersonName struct {
	FullName   string `json:"full_name"`
	GivenName  string `json:"given_name"`
	FamilyName string `json:"family_name"`
	MiddleName string `json:"middle_name,omitempty"`
	NameOnDoc  string `json:"name_on_document,omitempty"` // As it appears on travel document
}

// Passport represents passport information
type Passport struct {
	Number        string    `json:"number"`
	Country       string    `json:"country"` // ISO 3166-1 alpha-3 or alpha-2
	IssueDate     Date      `json:"issue_date"`
	ExpiryDate    Date      `json:"expiry_date"`
	IssueCountry  string    `json:"issue_country"`
	Nationality   string    `json:"nationality"`
	MRZCode       string    `json:"mrz_code,omitempty"` // Full MRZ for verification
	IsPrimary     bool      `json:"is_primary"`
	Type          string    `json:"type"` // P (passport), D (diplomatic), etc.
}

// NationalID represents a national identity document
type NationalID struct {
	Number      string   `json:"number"`
	Country     string   `json:"country"`
	IssueDate   Date     `json:"issue_date"`
	ExpiryDate  Date     `json:"expiry_date"`
	IDType      string   `json:"id_type"` // ID, driver's license, etc.
	FrontImage  string   `json:"front_image,omitempty"` // DocumentRef ID
	BackImage   string   `json:"back_image,omitempty"`  // DocumentRef ID
}

// Date represents a date (YYYY-MM-DD)
type Date struct {
	Year  int `json:"year"`
	Month int `json:"month"`
	Day   int `json:"day"`
}

// Gender represents gender
type Gender string

const (
	GenderMale    Gender = "M"
	GenderFemale  Gender = "F"
	GenderOther   Gender = "X"
	GenderUnknown Gender = "U"
)

// Photo represents a photo document
type Photo struct {
	DocumentRefID string `json:"document_ref_id"`
	Width        int    `json:"width"`
	Height       int    `json:"height"`
	MimeType     string `json:"mime_type"`
}

// Contact represents contact information
type Contact struct {
	Email string `json:"email"`
	Phone string `json:"phone"`
	// Secondary contact
	Email2 string `json:"email_2,omitempty"`
	Phone2 string `json:"phone_2,omitempty"`
}

// Address represents a physical address
type Address struct {
	Street       string `json:"street"`
	Unit         string `json:"unit,omitempty"`
	City         string `json:"city"`
	State        string `json:"state"`
	PostalCode   string `json:"postal_code"`
	Country      string `json:"country"`
	AddressOnDoc string `json:"address_on_document,omitempty"` // As on document
}

// EmergencyContact represents an emergency contact
type EmergencyContact struct {
	Name       PersonName `json:"name"`
	Relationship string `json:"relationship"`
	Phone      string    `json:"phone"`
	Email      string    `json:"email,omitempty"`
}

// Travel contains travel-related information
type Travel struct {
	PrimaryPassport    *Passport        `json:"primary_passport,omitempty"`
	SecondaryPassports []Passport       `json:"secondary_passports,omitempty"`
	VisaHistory        []Visa           `json:"visa_history,omitempty"`
	TravelHistory      []TravelEntry    `json:"travel_history,omitempty"`
	CountryPreferences []CountryPref    `json:"country_preferences,omitempty"`
}

// Visa represents a visa
type Visa struct {
	Number       string `json:"number"`
	Type         string `json:"type"` // Tourist, Business, Student, Work, Transit
	Country      string `json:"country"`
	IssueDate    Date   `json:"issue_date"`
	ExpiryDate   Date   `json:"expiry_date"`
	Entries      string `json:"entries"` // Single, Double, Multiple
	PortOfEntry  string `json:"port_of_entry,omitempty"`
	MRZCode      string `json:"mrz_code,omitempty"`
}

// TravelEntry represents a travel record
type TravelEntry struct {
	EntryDate   Date   `json:"entry_date"`
	ExitDate    Date   `json:"exit_date,omitempty"`
	Country     string `json:"country"`
	City        string `json:"city,omitempty"`
	Purpose     string `json:"purpose"`
	PortOfEntry string `json:"port_of_entry,omitempty"`
}

// CountryPref represents travel preferences for a country
type CountryPref struct {
	Country     string   `json:"country"`
	Preferences []string `json:"preferences"` // Visa requirements, embassy locations, etc.
}

// Financial contains financial information
type Financial struct {
	BankAccounts []BankAccount `json:"bank_accounts,omitempty"`
	Cards         []CardInfo    `json:"cards,omitempty"`
	TaxIDs        []TaxID       `json:"tax_ids,omitempty"`
}

// BankAccount represents bank account information
type BankAccount struct {
	BankName     string `json:"bank_name"`
	AccountNumber string `json:"account_number"`
	IBAN         string `json:"iban,omitempty"`
	SWIFT        string `json:"swift,omitempty"`
	AccountType  string `json:"account_type"` // Checking, Savings, etc.
	Currency     string `json:"currency"`
}

// CardInfo represents payment card information
type CardInfo struct {
	CardNumber  string `json:"card_number"` // Last 4 digits only for storage
	Cardholder  string `json:"cardholder"`
	ExpiryMonth int    `json:"expiry_month"`
	ExpiryYear  int    `json:"expiry_year"`
	CardType    string `json:"card_type"` // Visa, Mastercard, etc.
	BillingAddr Address `json:"billing_address"`
}

// TaxID represents tax identification
type TaxID struct {
	Country  string `json:"country"`
	Type     string `json:"type"` // SSN, ITIN, TIN, etc.
	Number   string `json:"number"`
	ExpiryDate Date `json:"expiry_date,omitempty"`
}

// Professional contains professional information
type Professional struct {
	Education     []Education     `json:"education,omitempty"`
	Employments   []Employment    `json:"employments,omitempty"`
	Languages     []Language      `json:"languages,omitempty"`
	Certifications []Certification `json:"certifications,omitempty"`
}

// Education represents educational background
type Education struct {
	Institution string `json:"institution"`
	Degree      string `json:"degree"`
	Field       string `json:"field"`
	StartDate   Date   `json:"start_date"`
	EndDate     Date   `json:"end_date"`
	Country     string `json:"country"`
}

// Employment represents employment history
type Employment struct {
	Company     string    `json:"company"`
	Title       string    `json:"title"`
	Description string    `json:"description,omitempty"`
	StartDate   Date      `json:"start_date"`
	EndDate     Date      `json:"end_date,omitempty"`
	Current     bool      `json:"current"`
	Country     string    `json:"country"`
}

// Language represents language proficiency
type Language struct {
	Code       string `json:"code"` // ISO 639-1
	Name       string `json:"name"`
	Proficiency string `json:"proficiency"` // Native, Fluent, Professional, etc.
}

// Certification represents professional certifications
type Certification struct {
	Name         string `json:"name"`
	Issuer       string `json:"issuer"`
	IssueDate    Date   `json:"issue_date"`
	ExpiryDate   Date   `json:"expiry_date,omitempty"`
	LicenseNumber string `json:"license_number,omitempty"`
}

// Preferences contains user preferences
type Preferences struct {
	MealPreference     string   `json:"meal_preference"`
	SeatPreference     string   `json:"seat_preference"`
	TravelCompanions   []string `json:"travel_companions,omitempty"`
	NotificationPrefs  NotificationPrefs `json:"notification_prefs"`
	FrequentFlyer      []FrequentFlyer `json:"frequent_flyer,omitempty"`
	HotelPreferences   []HotelPref `json:"hotel_preferences,omitempty"`
}

// NotificationPrefs represents notification preferences
type NotificationPrefs struct {
	Email    bool `json:"email"`
	SMS      bool `json:"sms"`
	Push     bool `json:"push"`
}

// FrequentFlyer represents frequent flyer program membership
type FrequentFlyer struct {
	ProgramName string `json:"program_name"`
	Number      string `json:"number"`
	Airline     string `json:"airline"`
	Tier        string `json:"tier,omitempty"`
}

// HotelPref represents hotel preferences
type HotelPref struct {
	Chain      string `json:"chain"`
	MemberNumber string `json:"member_number"`
	Tier       string `json:"tier,omitempty"`
}

// DocumentRef references a stored document
type DocumentRef struct {
	ID          string `json:"id"`
	DocType     string `json:"doc_type"` // Passport, ID, Visa, Photo, Other
	Title       string `json:"title"`
	Description string `json:"description,omitempty"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
	SourcePath  string `json:"source_path,omitempty"` // Original file path
	MRZData     string `json:"mrz_data,omitempty"` // Extracted MRZ if applicable
	Confidence  int    `json:"confidence,omitempty"` // OCR confidence 0-100
}

// ProfileMetadata contains system metadata
type ProfileMetadata struct {
	Locale    string   `json:"locale"`
	Timezone  string   `json:"timezone"`
	Tags      []string `json:"tags,omitempty"`
	CreatedBy string   `json:"created_by"`
	ModifiedBy string  `json:"modified_by"`
}

// NewProfile creates a new profile with defaults
func NewProfile(profileID string) *SuperProfile {
	now := time.Now()
	return &SuperProfile{
		Version:   "1.0",
		ProfileID: profileID,
		CreatedAt: now,
		UpdatedAt: now,
		Metadata: ProfileMetadata{
			Locale:   "en-US",
			Timezone: "UTC",
		},
	}
}
