# Object Templates

Templates are preset object structures that help you quickly create records in standard formats. SoloSoul includes several built-in templates and supports saving any object as a custom template.

## Template Sources

- **System templates**: Built-in standard templates that expand with app updates. They are read-only.
- **User templates**: Templates you create yourself. Stored locally; can be edited or deleted anytime.

## Built-in Templates

SoloSoul ships with 10 system templates:

| Template | Section | Use Case |
|----------|---------|----------|
| Identity (identity) | Identity | Basic personal information |
| ID Card (id_card) | Identity | ID card details |
| Address (address) | Identity | Frequently used addresses |
| Contact (contact) | Identity | Contact information |
| Passport (passport) | Travel | Passport details |
| Visa (visa) | Travel | Visa information |
| Bank Account (bank) | Financial | Bank account details |
| Bank Card (card) | Financial | Bank card |
| Education (education) | Professional | Education history |
| Employment (employment) | Professional | Work experience |

## Using Templates

1. Enter the object editor (when creating a new object)
2. Select a template type at the top
3. The system automatically expands the corresponding field form and shows a template preview below (field types, required/sensitive indicators)
4. Fill in and save

<!--STEPPER Use the Passport template-->
1. Go to the **Travel** workspace
2. Click **+ Create**
3. Select the **Passport** template
4. Review the field preview to see required fields
5. Fill in passport number, nationality, issue date, expiry date
6. Click **Save**
<!--/STEPPER-->

## Creating and Editing Templates

You can create your own templates on the **Template Manager** page:

1. Go to **Settings → Template Manager**
2. Click **New Template**
3. Set a template name and its type (section)
4. Add fields, and set the type and sensitivity level for each field
5. Save the template

After creation, you can edit template fields or delete the template at any time.

<!--TIP-->
Custom templates are stored locally only. They are never uploaded to any server.
<!--/TIP-->

## Template-Object Association

When creating an object, you can associate it with a template (system or user). This association is recorded in the object data, making it easy to trace which template an object was created from. Even if the template is later modified or deleted, the object's own data and field definitions (`__fields`) remain intact.

## Related Docs

<!--CARDS-->
- [Object Management](objects.md) — Create, edit and delete objects
- [Workspace](workspace.md) — Organize objects and custom pages
- [Attachment Management](attachments.md) — Manage files and images
<!--/CARDS-->

