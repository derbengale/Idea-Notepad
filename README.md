![image](https://github.com/derbengale/Idea-Notepad/assets/28060331/65decf16-bcef-4682-8c8f-8d09fa405315)

# Idea Notepad

Idea Notepad is a productivity tool designed to help users document and sort their ideas, hypotheses, and experiments.

## About

The application is developed using PySide6, a Python binding of the Qt application framework. It provides a graphical interface where users can input their ideas or notes, assign them an importance score, complexity score, and time consumption score. These scores are used to calculate a productivity score which is a way of prioritizing these ideas or notes. 

This tool was designed in a scientific context to facilitate the organization and prioritization of ideas, hypotheses, and experiments. However, it can be easily repurposed for other contexts where such organization and prioritization can be beneficial.

## Features

- Add new ideas, hypotheses or experiments with an importance score, complexity score, and time consumption score.
- Filter entries based on the type (All, Ideas, Hypotheses, Experiments).
- Edit existing entries.
- Delete entries.
- Undo the last deletion.
- Save changes automatically to a JSON file.
- Load previous entries from a JSON file.

## Dependencies

Idea Notepad requires PySide6 to run. You can install it via pip:

    pip install PySide6

How to Run

Clone this repository, navigate to the directory containing the main.py file, and run the following command:

    python Notepad.py

## Usage

- Start by selecting the type of the entry you're about to add from the dropdown menu.
- Fill in the text fields for "Idea" and "Notes".
- Use the spin boxes to specify the "Importance", "Complexity", and "Time Consumption" of the entry.
- Click "Add Entry" to add the entry to the table.
- You can click on any row in the table to select it. The input fields will populate with the data from the selected row.
- If you make any changes to an existing entry, the "Save Changes" button will become enabled. Click it to save your changes.
- You can delete a selected entry by clicking "Delete Entry". If you accidentally delete an entry, you can click "Undo Last Deletion" to restore it.

Please note that it uses the file `notes.json` to persist the state of the notepad between sessions. This file will be created in the same directory as your script if it does not exist.

## Contributions

Contributions to the Idea Notepad are welcome. Please feel free to open a pull request or issue on GitHub.
