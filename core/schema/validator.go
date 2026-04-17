package schema

import (
	"errors"
	"fmt"
	"regexp"
	"time"
)

// ValidationError represents a schema validation error
type ValidationError struct {
	Field   string `json:"field"`
	Message string `json:"message"`
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("%s: %s", e.Field, e.Message)
}

// Validator validates profile data
type Validator struct{}

// NewValidator creates a new validator
func NewValidator() *Validator {
	return &Validator{}
}

// Validate validates a complete profile
func (v *Validator) Validate(profile *SuperProfile) []ValidationError {
	var errs []ValidationError

	if profile.ProfileID == "" {
		errs = append(errs, ValidationError{Field: "profile_id", Message: "required"})
	}

	// Validate Identity
	errs = append(errs, v.validateIdentity(&profile.Identity)...)

	// Validate Travel
	errs = append(errs, v.validateTravel(&profile.Travel)...)

	// Validate Financial
	errs = append(errs, v.validateFinancial(&profile.Financial)...)

	// Validate Professional
	errs = append(errs, v.validateProfessional(&profile.Professional)...)

	return errs
}

func (v *Validator) validateIdentity(identity *Identity) []ValidationError {
	var errs []ValidationError

	// All identity fields are optional - only validate format if provided
	if identity.DateOfBirth.Year != 0 && !v.isValidDate(identity.DateOfBirth) {
		errs = append(errs, ValidationError{Field: "identity.date_of_birth", Message: "invalid date"})
	}

	errs = append(errs, v.validateContact(&identity.Contact)...)

	return errs
}

func (v *Validator) validateContact(contact *Contact) []ValidationError {
	var errs []ValidationError

	if contact.Email != "" && !v.isValidEmail(contact.Email) {
		errs = append(errs, ValidationError{Field: "identity.contact.email", Message: "invalid email format"})
	}

	if contact.Phone != "" && !v.isValidPhone(contact.Phone) {
		errs = append(errs, ValidationError{Field: "identity.contact.phone", Message: "invalid phone format"})
	}

	return errs
}

func (v *Validator) validateTravel(travel *Travel) []ValidationError {
	var errs []ValidationError

	// Validate primary passport
	if travel.PrimaryPassport != nil {
		errs = append(errs, v.validatePassport(travel.PrimaryPassport)...)
	}

	// Validate secondary passports
	for i, passport := range travel.SecondaryPassports {
		errs = append(errs, v.validatePassport(&passport)...)
		if passport.Number != "" && travel.PrimaryPassport != nil && passport.Number == travel.PrimaryPassport.Number {
			errs = append(errs, ValidationError{
				Field:   fmt.Sprintf("travel.secondary_passports[%d].number", i),
				Message: "duplicate of primary passport",
			})
		}
	}

	return errs
}

func (v *Validator) validatePassport(passport *Passport) []ValidationError {
	var errs []ValidationError

	// All passport fields are optional - only validate format if provided
	if passport.IssueDate.Year != 0 && !v.isValidDate(passport.IssueDate) {
		errs = append(errs, ValidationError{Field: "travel.passport.issue_date", Message: "invalid date"})
	}

	if passport.ExpiryDate.Year != 0 && !v.isValidDate(passport.ExpiryDate) {
		errs = append(errs, ValidationError{Field: "travel.passport.expiry_date", Message: "invalid date"})
	}

	if passport.IssueDate.Year != 0 && passport.ExpiryDate.Year != 0 && v.isValidDate(passport.IssueDate) && v.isValidDate(passport.ExpiryDate) {
		issue := time.Date(passport.IssueDate.Year, time.Month(passport.IssueDate.Month), passport.IssueDate.Day, 0, 0, 0, 0, time.UTC)
		expiry := time.Date(passport.ExpiryDate.Year, time.Month(passport.ExpiryDate.Month), passport.ExpiryDate.Day, 0, 0, 0, 0, time.UTC)
		if !expiry.After(issue) {
			errs = append(errs, ValidationError{Field: "travel.passport.expiry_date", Message: "must be after issue date"})
		}
	}

	return errs
}

func (v *Validator) validateFinancial(financial *Financial) []ValidationError {
	var errs []ValidationError

	for i, card := range financial.Cards {
		if card.ExpiryYear > 0 && card.ExpiryYear < time.Now().Year() {
			errs = append(errs, ValidationError{
				Field:   fmt.Sprintf("financial.cards[%d].expiry_year", i),
				Message: "card is expired",
			})
		}
	}

	return errs
}

func (v *Validator) validateProfessional(prof *Professional) []ValidationError {
	var errs []ValidationError

	return errs
}

// Helper validators

var (
	emailRegex = regexp.MustCompile(`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`)
	phoneRegex = regexp.MustCompile(`^[+]?[0-9\s\-\(\)]{7,20}$`)
)

func (v *Validator) isValidEmail(email string) bool {
	return emailRegex.MatchString(email)
}

func (v *Validator) isValidPhone(phone string) bool {
	return phoneRegex.MatchString(phone)
}

func (v *Validator) isValidDate(d Date) bool {
	if d.Year == 0 || d.Month == 0 || d.Day == 0 {
		return false
	}
	if d.Month < 1 || d.Month > 12 {
		return false
	}
	if d.Day < 1 || d.Day > 31 {
		return false
	}
	if d.Year < 1800 || d.Year > 2200 {
		return false
	}
	return true
}

// FieldValidator validates individual fields
func ValidateField(fieldPath string, value interface{}) error {
	switch fieldPath {
	case "identity.contact.email":
		if email, ok := value.(string); ok && !NewValidator().isValidEmail(email) {
			return errors.New("invalid email format")
		}
	case "identity.contact.phone":
		if phone, ok := value.(string); ok && !NewValidator().isValidPhone(phone) {
			return errors.New("invalid phone format")
		}
	}
	return nil
}
