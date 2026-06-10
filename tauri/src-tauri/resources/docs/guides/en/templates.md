# Object Templates

Templates are preset object structures that help you quickly create records in standard formats. SoloSoul includes several built-in templates and supports saving any object as a custom template.

## Template Sources

- **System templates**: Built-in standard templates that expand with app updates. They are read-only.
- **User templates**: Custom templates saved from existing objects. Stored locally and can be deleted anytime.

## Built-in Templates

| Template | Section | Use Case |
|----------|---------|----------|
| Identity | Identity | Basic personal information |
| Passport | Travel | Passport details |
| Visa | Travel | Visa information |
| Bank Account | Financial | Bank account details |
| Card | Financial | Credit / debit card |
| Education | Professional | Education history |
| Employment | Professional | Work experience |

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

## Saving Custom Templates

If you create an object with a structure you reuse often, save it as a template:

1. Open the object's edit page
2. Click **Save as Template**
3. Enter a template name
4. Confirm

<!--TIP-->
Custom templates are stored locally only. They are never uploaded to any server.
<!--/TIP-->

## Template-Object Association

When creating an object, you can associate it with a template (system or user). This association is recorded in the object data, making it easy to trace which template an object was created from. Removing the association does not affect the object's data.

## Related Docs

<!--CARDS-->
- [Object Management](objects.md) — Create, edit and delete objects
- [Workspace](workspace.md) — Organize objects and custom pages
- [Attachment Management](attachments.md) — Manage files and images
<!--/CARDS-->

