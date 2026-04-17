package schema

import (
	"testing"
	"time"
)

func TestNewProfile(t *testing.T) {
	profileID := "test-profile-001"
	profile := NewProfile(profileID)

	if profile.ProfileID != profileID {
		t.Errorf("ProfileID = %q, want %q", profile.ProfileID, profileID)
	}

	if profile.Version != "1.0" {
		t.Errorf("Version = %q, want %q", profile.Version, "1.0")
	}

	if profile.CreatedAt.IsZero() {
		t.Error("CreatedAt should not be zero")
	}

	if profile.UpdatedAt.IsZero() {
		t.Error("UpdatedAt should not be zero")
	}

	if profile.Metadata.Locale != "en-US" {
		t.Errorf("Metadata.Locale = %q, want %q", profile.Metadata.Locale, "en-US")
	}

	if profile.Metadata.Timezone != "UTC" {
		t.Errorf("Metadata.Timezone = %q, want %q", profile.Metadata.Timezone, "UTC")
	}
}

func TestNewProfile_Timestamps(t *testing.T) {
	before := time.Now()
	profile := NewProfile("test")
	after := time.Now()

	if profile.CreatedAt.Before(before) || profile.CreatedAt.After(after) {
		t.Errorf("CreatedAt = %v, want between %v and %v", profile.CreatedAt, before, after)
	}

	if profile.UpdatedAt.Before(before) || profile.UpdatedAt.After(after) {
		t.Errorf("UpdatedAt = %v, want between %v and %v", profile.UpdatedAt, before, after)
	}
}

func TestNewProfile_EmptyProfileID(t *testing.T) {
	profile := NewProfile("")
	if profile.ProfileID != "" {
		t.Errorf("ProfileID = %q, want empty string", profile.ProfileID)
	}
}

func TestNewProfile_IdentityFields(t *testing.T) {
	profile := NewProfile("test")
	// Identity should be zero value
	if profile.Identity.FullName.FullName != "" {
		t.Errorf("Identity.FullName.FullName = %q, want empty", profile.Identity.FullName.FullName)
	}
	if profile.Identity.DateOfBirth.Year != 0 {
		t.Errorf("Identity.DateOfBirth.Year = %d, want 0", profile.Identity.DateOfBirth.Year)
	}
	if profile.Identity.Contact.Email != "" {
		t.Errorf("Identity.Contact.Email = %q, want empty", profile.Identity.Contact.Email)
	}
}

func TestNewProfile_TravelFields(t *testing.T) {
	profile := NewProfile("test")
	// Travel should be zero value
	if profile.Travel.PrimaryPassport != nil {
		t.Error("Travel.PrimaryPassport should be nil")
	}
	if profile.Travel.VisaHistory != nil {
		t.Error("Travel.VisaHistory should be nil")
	}
}

func TestNewProfile_FinancialFields(t *testing.T) {
	profile := NewProfile("test")
	// Financial should be zero value
	if profile.Financial.BankAccounts != nil {
		t.Error("Financial.BankAccounts should be nil")
	}
	if profile.Financial.Cards != nil {
		t.Error("Financial.Cards should be nil")
	}
}

func TestNewProfile_ProfessionalFields(t *testing.T) {
	profile := NewProfile("test")
	// Professional should be zero value
	if profile.Professional.Education != nil {
		t.Error("Professional.Education should be nil")
	}
	if profile.Professional.Languages != nil {
		t.Error("Professional.Languages should be nil")
	}
}

func TestNewProfile_PreferencesFields(t *testing.T) {
	profile := NewProfile("test")
	// Preferences should have default values
	if profile.Preferences.MealPreference != "" {
		t.Errorf("Preferences.MealPreference = %q, want empty", profile.Preferences.MealPreference)
	}
	if profile.Preferences.NotificationPrefs.Email != false {
		t.Error("Preferences.NotificationPrefs.Email should be false by default")
	}
}

func TestNewProfile_Documents(t *testing.T) {
	profile := NewProfile("test")
	if profile.Documents != nil {
		t.Error("Documents should be nil by default")
	}
}

func TestNewProfile_Metadata(t *testing.T) {
	profile := NewProfile("test")

	// Check metadata defaults
	if profile.Metadata.Locale != "en-US" {
		t.Errorf("Metadata.Locale = %q, want en-US", profile.Metadata.Locale)
	}
	if profile.Metadata.Timezone != "UTC" {
		t.Errorf("Metadata.Timezone = %q, want UTC", profile.Metadata.Timezone)
	}
	if profile.Metadata.Tags != nil {
		t.Error("Metadata.Tags should be nil by default")
	}
}

func TestFieldTypeConstants(t *testing.T) {
	// Verify field type constants are distinct
	types := []FieldType{
		FieldTypeString,
		FieldTypeInt,
		FieldTypeBool,
		FieldTypeDate,
		FieldTypeTimestamp,
		FieldTypeBinary,
		FieldTypeEnum,
		FieldTypeCompound,
		FieldTypeList,
		FieldTypeSensitive,
	}

	for i, ft1 := range types {
		for j, ft2 := range types {
			if i != j && ft1 == ft2 {
				t.Errorf("FieldType constants at indices %d and %d should be distinct", i, j)
			}
		}
	}

	// Verify iota values
	if FieldTypeString != 0 {
		t.Errorf("FieldTypeString = %d, want 0", FieldTypeString)
	}
	if FieldTypeSensitive != 9 {
		t.Errorf("FieldTypeSensitive = %d, want 9", FieldTypeSensitive)
	}
}

func TestEncryptionLevelConstants(t *testing.T) {
	// Verify encryption level constants are distinct
	levels := []EncryptionLevel{
		EncryptAtRest,
		EncryptAlways,
		EncryptNever,
	}

	for i, el1 := range levels {
		for j, el2 := range levels {
			if i != j && el1 == el2 {
				t.Errorf("EncryptionLevel constants at indices %d and %d should be distinct", i, j)
			}
		}
	}

	// Verify iota values
	if EncryptAtRest != 0 {
		t.Errorf("EncryptAtRest = %d, want 0", EncryptAtRest)
	}
	if EncryptAlways != 1 {
		t.Errorf("EncryptAlways = %d, want 1", EncryptAlways)
	}
	if EncryptNever != 2 {
		t.Errorf("EncryptNever = %d, want 2", EncryptNever)
	}
}

func TestGenderConstants(t *testing.T) {
	genders := []Gender{
		GenderMale,
		GenderFemale,
		GenderOther,
		GenderUnknown,
	}

	// Verify values
	if GenderMale != "M" {
		t.Errorf("GenderMale = %q, want M", GenderMale)
	}
	if GenderFemale != "F" {
		t.Errorf("GenderFemale = %q, want F", GenderFemale)
	}
	if GenderOther != "X" {
		t.Errorf("GenderOther = %q, want X", GenderOther)
	}
	if GenderUnknown != "U" {
		t.Errorf("GenderUnknown = %q, want U", GenderUnknown)
	}

	// Verify distinctness
	for i, g1 := range genders {
		for j, g2 := range genders {
			if i != j && g1 == g2 {
				t.Errorf("Gender constants at indices %d and %d should be distinct", i, j)
			}
		}
	}
}

func TestDateStruct(t *testing.T) {
	d := Date{
		Year:  2024,
		Month: 1,
		Day:   15,
	}

	if d.Year != 2024 {
		t.Errorf("Date.Year = %d, want 2024", d.Year)
	}
	if d.Month != 1 {
		t.Errorf("Date.Month = %d, want 1", d.Month)
	}
	if d.Day != 15 {
		t.Errorf("Date.Day = %d, want 15", d.Day)
	}
}
