#!/bin/bash
cd /Users/zzc/PycharmProjects/SoloSoul_code
git add tauri/src/components/TemplateFieldInput.tsx tauri/package.json
git rm tauri/src/components/forms/DatePicker.tsx tauri/src/components/forms/DatePicker.module.css tauri/src/components/forms/DatePicker.test.tsx tauri/src/components/ui/DropdownSelect.tsx tauri/src/components/ui/DropdownSelect.module.css
git commit -m 'fix(P0-3a): replace DatePicker + DropdownSelect with native input[type=date] (-509 lines)

Replace custom ~286-line DatePicker (with DropdownSelect for year/month),
~223-line DropdownSelect, and date-fns dependency with native
<input type="date"> and <input type="datetime-local"> elements.
Both cases now use the existing styles.input CSS class.'
git push origin master
