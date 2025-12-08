
# File Sets

# Creating a File Set

## Creating a File Set with URL

## Creating a File Set with Local Files

# Deleting a File Set

Starting point: a release has been created
- Select a release
- Press "Edit Release" button
    - "Release Form" window opens
- Press "Select File Set" button
    - "File Set Selector" window opens
- Select File Type from dropdown, for example "Manual"
- Press "Create File Set" button 
    - "Create File Set" window opens
    - The previously selected File Type is already selected in the dropdown
- Press "Open File Picker" button 
    - File Picker window opens
- Browse to and select a document file on local system
    - A label with the file name appears below to the "Open File Picker" and "Download from URL" buttons:
      e.g. "Selected file: /tmp/manual.pdf"
    - File name should appear in files list below with checkbox selected
    - "File Set File Name" field is auto-populated with the file name (file set file name can be edited here if desired)
    - "File Set Display Name" field is auto-populated with the file name without extension (file set name can be edited here if desired)
- Press "Create File Set" button
    - File Set is created and selected in the File Set Selector window (TODO: do another test case for deleting a file set now)
- Press "Select File Set" button
    - File Set is now selected in the Release Form window 
- Press "Submit Release" button
    - Release is updated with the new File Set, if the file type was manual it appears in the "Document File Sets" list

- Press "Edit Release" button
    - "Release Form" window opens
- Select the created File Set in the "File Sets" list
- Press "Unlink File Set" button (TODO: add also delete file set button here)
    - File Set disappears from the "File Sets" list 
- Press "Submit Release" button
    - File Set no longer appears in the Release details, for example in the "Document File Sets" list

- Press "Edit Release" button
    - "Release Form" window opens
- Press "Select File Set" button
    - "File Set Selector" window opens
- Ensure that the File Type dropdown has the correct type selected (same as when the file set was created)
    - File set appears in the list
- Select the File Set in the list
- Press "Delete File Set" button
    - TODO: implement this - Confirmation dialog appears
- Confirm deletion
    - A message dialog appears confirming deletion 
    - File Set no longer appears in the list


