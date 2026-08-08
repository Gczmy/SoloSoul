# Object Types & Sections

In SoloSoul, every object has a **type (Collection Type)** that determines which **section (Collection)** it belongs to.

## Types and Sections

Built-in types include:

| Type | Section |
|------|---------|
| identity (身份信息) | Identity |
| id_card (身份证), address (地址), contact (联系人) | Identity |
| passport (护照), visa (签证) | Travel |
| bank (银行账户), card (银行卡) | Financial |
| education (教育经历), employment (工作经历) | Professional |
| document (文档, e.g. objects imported from OCR) | Document |

- Each object picks a type at creation and is grouped into the corresponding section
- The home page and sidebar show sections as cards; entering a section lists all objects of that type
- Objects on custom pages keep their original type membership

<!--TIP-->
The type label is shown on object cards and in the detail view, making it easy to identify the section an object belongs to.
<!--/TIP-->

## Types and Templates

- **Templates** define a type's field structure (properties, sensitivity, etc.) — see [Object Templates](templates.md)
- When creating an object, choose a template to get the type's preset field form
- Objects of the same type share the template fields, but each object's data is independent

## Related Docs

<!--CARDS-->
- [Object Management](objects.md) — Object basics
- [Workspace](workspace.md) — Sections and custom pages
- [Object Templates](templates.md) — Type field structure
<!--/CARDS-->
