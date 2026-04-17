#!/usr/bin/env python3
"""
Generate test data for SoloSoul migration tests.

Generates native/test_data/test_v1.json with all 11 categories:
- identity: idCards, contact.entries, addresses
- travel: passports, visas, travelHistory
- financial: bankAccounts, cards, taxIds
- professional: education, employment, skills, languages

Uses camelCase field names to match the Dart/Rust schema.
"""

import json
import os
from datetime import datetime, timedelta

OUTPUT_DIR = os.path.join(os.path.dirname(__file__), '..', 'test_data')
OUTPUT_FILE = os.path.join(OUTPUT_DIR, 'test_v1.json')


def generate_test_v1():
    """Generate a complete V1 profile with all 11 categories."""
    now = datetime.now()

    # Identity data
    identity = {
        "full_name": "Zhang Wei",
        "given_name": "Wei",
        "family_name": "Zhang",
        "date_of_birth": "1990-05-15",
        "gender": "male",
        "nationality": "Chinese",
        "id_cards": [
            {
                "label": "Main ID",
                "number": "110101199005151234",
                "issue_date": "2015-03-20",
                "expiry_date": "2025-03-20",
                "holder_name": "Zhang Wei",
                "country": "China",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "label": "Secondary",
                "number": "110101199005151235",
                "issue_date": "2018-06-10",
                "expiry_date": "2028-06-10",
                "holder_name": "Zhang Wei",
                "country": "China",
                "is_deleted": False,
                "deleted_at": None
            }
        ],
        "contact": {
            "entries": [
                {
                    "label": "Personal",
                    "type": "email",
                    "value": "zhangwei1990@gmail.com",
                    "is_deleted": False,
                    "deleted_at": None
                },
                {
                    "label": "Work",
                    "type": "email",
                    "value": "wei.zhang@techcorp.com",
                    "is_deleted": False,
                    "deleted_at": None
                },
                {
                    "label": "Mobile",
                    "type": "phone",
                    "value": "+86 138 0013 8000",
                    "is_deleted": False,
                    "deleted_at": None
                },
                {
                    "label": "Emergency",
                    "type": "mobile",
                    "value": "+86 139 0013 9000",
                    "is_deleted": False,
                    "deleted_at": None
                }
            ]
        },
        "addresses": [
            {
                "label": "Home",
                "street": "123 Sunshine Street, Chaoyang District",
                "city": "Beijing",
                "state": "Beijing",
                "postal_code": "100025",
                "country": "China",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "label": "Office",
                "street": "456 Innovation Road, Haidian District",
                "city": "Beijing",
                "state": "Beijing",
                "postal_code": "100089",
                "country": "China",
                "is_deleted": False,
                "deleted_at": None
            }
        ]
    }

    # Travel data
    travel = {
        "passports": [
            {
                "number": "E12345678",
                "country": "China",
                "issue_date": "2020-01-15",
                "expiry_date": "2030-01-15",
                "holder_name": "ZHANG WEI",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "number": "E87654321",
                "country": "China",
                "issue_date": "2015-06-20",
                "expiry_date": "2025-06-20",
                "holder_name": "ZHANG WEI",
                "is_deleted": True,
                "deleted_at": (now - timedelta(days=45)).isoformat()
            }
        ],
        "visas": [
            {
                "country": "United States",
                "visa_type": "B1/B2",
                "number": "V1234567",
                "issue_date": "2022-03-01",
                "expiry_date": "2032-03-01",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "country": "Japan",
                "visa_type": "Tourist",
                "number": "V7654321",
                "issue_date": "2023-07-15",
                "expiry_date": "2025-07-15",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "country": "Schengen",
                "visa_type": "C",
                "number": "V1122334",
                "issue_date": "2019-04-10",
                "expiry_date": "2024-04-10",
                "is_deleted": True,
                "deleted_at": (now - timedelta(days=60)).isoformat()
            }
        ],
        "travel_history": [
            {
                "destination": "Tokyo, Japan",
                "date": "2023-07-20",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "destination": "San Francisco, USA",
                "date": "2022-09-15",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "destination": "Paris, France",
                "date": "2019-05-10",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "destination": "Berlin, Germany",
                "date": "2019-04-20",
                "is_deleted": True,
                "deleted_at": (now - timedelta(days=90)).isoformat()
            }
        ]
    }

    # Financial data
    financial = {
        "bank_accounts": [
            {
                "bank_name": "Industrial and Commercial Bank of China",
                "account_number": "6222 1234 5678 9012",
                "currency": "CNY",
                "swift_bic": "ICBKCNBJ",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "bank_name": "HSBC Hong Kong",
                "account_number": "123 456 789",
                "currency": "HKD",
                "swift_bic": "HSBCHKHH",
                "is_deleted": False,
                "deleted_at": None
            }
        ],
        "cards": [
            {
                "card_number": "4532 1234 5678 9012",
                "card_type": "Visa",
                "expiry_date": "2026-12",
                "holder_name": "ZHANG WEI",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "card_number": "5412 3456 7890 1234",
                "card_type": "Mastercard",
                "expiry_date": "2025-08",
                "holder_name": "ZHANG WEI",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "card_number": "3782 822463 10005",
                "card_type": "American Express",
                "expiry_date": "2024-06",
                "holder_name": "ZHANG WEI",
                "is_deleted": True,
                "deleted_at": (now - timedelta(days=30)).isoformat()
            }
        ],
        "tax_ids": [
            {
                "tax_id_number": "110101199005151234",
                "tax_id_type": "National ID",
                "issuing_authority": "Beijing Public Security Bureau",
                "country": "China",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "tax_id_number": "A1234567",
                "tax_id_type": "SSN",
                "issuing_authority": "IRS",
                "country": "United States",
                "is_deleted": False,
                "deleted_at": None
            }
        ]
    }

    # Professional data
    professional = {
        "education": [
            {
                "institution": "Tsinghua University",
                "degree": "Bachelor of Engineering",
                "field": "Computer Science",
                "start_date": "2008-09",
                "end_date": "2012-06",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "institution": "Stanford University",
                "degree": "Master of Science",
                "field": "Artificial Intelligence",
                "start_date": "2012-09",
                "end_date": "2014-06",
                "is_deleted": False,
                "deleted_at": None
            }
        ],
        "employment": [
            {
                "company": "TechCorp Inc.",
                "position": "Senior Software Engineer",
                "start_date": "2014-07",
                "end_date": "2020-12",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "company": "StartupXYZ",
                "position": "Tech Lead",
                "start_date": "2021-01",
                "end_date": None,
                "is_deleted": False,
                "deleted_at": None
            }
        ],
        "skills": [
            {
                "name": "Python",
                "level": "Expert",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "name": "Machine Learning",
                "level": "Advanced",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "name": "Rust",
                "level": "Intermediate",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "name": "Project Management",
                "level": "Intermediate",
                "is_deleted": True,
                "deleted_at": (now - timedelta(days=120)).isoformat()
            }
        ],
        "languages": [
            {
                "name": "Mandarin Chinese",
                "proficiency": "Native",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "name": "English",
                "proficiency": "Fluent",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "name": "Japanese",
                "proficiency": "Basic",
                "is_deleted": False,
                "deleted_at": None
            },
            {
                "name": "German",
                "proficiency": "Beginner",
                "is_deleted": True,
                "deleted_at": (now - timedelta(days=200)).isoformat()
            }
        ]
    }

    profile_data = {
        "version": 1,
        "data": {
            "identity": identity,
            "travel": travel,
            "financial": financial,
            "professional": professional
        }
    }

    return profile_data


def main():
    """Generate test data file."""
    # Create output directory if it doesn't exist
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Generate test data
    test_data = generate_test_v1()

    # Write to file with pretty formatting
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        json.dump(test_data, f, indent=2, ensure_ascii=False)

    print(f"Generated test data: {OUTPUT_FILE}")
    print(f"  - Version: {test_data['version']}")
    print(f"  - Categories: identity, travel, financial, professional")
    print(f"  - ID Cards: {len(test_data['data']['identity']['id_cards'])}")
    print(f"  - Contact entries: {len(test_data['data']['identity']['contact']['entries'])}")
    print(f"  - Addresses: {len(test_data['data']['identity']['addresses'])}")
    print(f"  - Passports: {len(test_data['data']['travel']['passports'])}")
    print(f"  - Visas: {len(test_data['data']['travel']['visas'])}")
    print(f"  - Travel history: {len(test_data['data']['travel']['travel_history'])}")
    print(f"  - Bank accounts: {len(test_data['data']['financial']['bank_accounts'])}")
    print(f"  - Cards: {len(test_data['data']['financial']['cards'])}")
    print(f"  - Tax IDs: {len(test_data['data']['financial']['tax_ids'])}")
    print(f"  - Education: {len(test_data['data']['professional']['education'])}")
    print(f"  - Employment: {len(test_data['data']['professional']['employment'])}")
    print(f"  - Skills: {len(test_data['data']['professional']['skills'])}")
    print(f"  - Languages: {len(test_data['data']['professional']['languages'])}")

    # Calculate expected hash for fingerprint test
    import hashlib
    import json as json_module

    # Normalize JSON for consistent hashing
    normalized = json_module.dumps(test_data, sort_keys=True, separators=(',', ':'))
    hash_value = hashlib.sha256(normalized.encode()).hexdigest()
    print(f"\nSHA256 fingerprint: {hash_value}")
    print(f"  (Use this value in migration_fingerprint_test.dart)")


if __name__ == '__main__':
    main()
