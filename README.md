# Idea Notepad

Idea Notepad is a Python-based desktop application, designed to provide an intuitive interface for users to manage and explore their ideas, hypotheses, and experiments. I created it as a Template to work with JSON Files in another project.

![image](https://github.com/derbengale/Idea-Notepad/assets/28060331/0e408935-4cbc-4754-934e-f3338d65df17)



## Features

- **Entry Management**: The application allows users to add entries with corresponding importance, complexity, and time consumption metrics. Each entry also includes a section for additional notes.

- **Productivity Score Calculation**: Idea Notepad calculates a 'Productivity Score' for each entry, based on the provided importance, complexity, and time consumption. This feature enables users to gauge the efficiency of their ideas or hypotheses.

- **Data Storage**: All the information is stored in a JSON file, ensuring that the data is saved and can be retrieved for future use.

- **Interactive Table View**: The application features an interactive table view that presents all the entries. Users can filter the table based on the type of entry (Ideas, Hypotheses, Experiments, or All).

- **Entry Deletion and Undo**: The application supports the deletion of entries with the added functionality of undoing the last deletion if necessary.

## Technology Stack

A combination of PySide6 and Qt is used to build the application's GUI, making it both user-friendly and visually appealing.

- **PySide6**: Used for creating the application's graphical user interface.
- **Qt**: A cross-platform application framework used in conjunction with PySide6.

## Additional Features

The application also supports different font sizes to enhance readability.
