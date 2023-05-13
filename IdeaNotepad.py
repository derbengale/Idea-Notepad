from PySide6.QtWidgets import QApplication, QLabel, QVBoxLayout, QWidget, QTextEdit, QTableView, QPushButton, QSpinBox, QComboBox,QHeaderView
from PySide6.QtGui import QStandardItemModel, QStandardItem, QFont
from PySide6.QtCore import Qt
import json
import os

class MainWindow(QWidget):
    def __init__(self):
        self.hide_rows = True
        self.hide_cols = True
        self.selected_row = None
        self.deleted_rows = [] # This stack is used to track the deleted rows for undo operation
        super().__init__()
        self.filename = 'notes.json'
        # os.remove(self.filename)
        self.layout = QVBoxLayout(self)

        # Erstellung der QTextEdit Zeile mit dem Label "Idea"

        self.type_filter = QComboBox()
        self.type_filter.addItem("All")  # Eintrag für alle Typen
        self.type_filter.addItem("Ideas")  # Eintrag für Ideen
        self.type_filter.addItem("Hypotheses")  # Eintrag für Ideen
        self.type_filter.addItem("Experiments")  # Eintrag für Ideen
        
        self.type_filter.currentTextChanged.connect(self.filter_table)
        self.layout.addWidget(self.type_filter)
        self.layout.addWidget(QLabel("Idea"))
        self.idea_input = QTextEdit("Keine")
        self.layout.addWidget(self.idea_input)

        self.layout.addWidget(QLabel("Notes"))
        self.note_input = QTextEdit("Keine")
        self.layout.addWidget(self.note_input)

        # Erstellung der Spinboxen mit den Labels "Importance", "Complexity" und "Time Consumption"
        self.layout.addWidget(QLabel("Importance"))
        self.importance_input = QSpinBox()
        self.importance_input.setRange(1, 3)
        self.layout.addWidget(self.importance_input)

        self.layout.addWidget(QLabel("Complexity"))
        self.complexity_input = QSpinBox()
        self.complexity_input.setRange(1, 3)
        self.layout.addWidget(self.complexity_input)

        self.layout.addWidget(QLabel("Time Consumption"))
        self.time_input = QSpinBox()
        self.time_input.setRange(1, 6)
        self.layout.addWidget(self.time_input)

        # Erstellung des QPushButton mit dem Label "Add"
        self.add_button = QPushButton("Add")
        self.add_button.clicked.connect(self.add_to_table)
        self.layout.addWidget(self.add_button)

        # Erstellung des QTableView Objekts
        self.table = QTableView()
        self.table.verticalHeader().setVisible(False)
        self.table.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.table.setSortingEnabled(True)

        self.table.clicked.connect(self.select_row)

        self.delete_button = QPushButton("Delete Entry")
        self.delete_button.clicked.connect(self.delete_row)
        self.layout.addWidget(self.delete_button)

        self.undo_button = QPushButton("Undo Last Deletion") # This button is used to undo the last deletion
        self.undo_button.clicked.connect(self.undo_last_deletion)
        self.layout.addWidget(self.undo_button)

        self.model = QStandardItemModel(0, 6)  # Anzahl der Zeilen und Spalten
        self.model.setHorizontalHeaderLabels(['_Type','Idea', 'Importance', 'Complexity', 'Time Consumption', 'Productivity Score', 'Notes'])
        self.table.setModel(self.model)
        self.layout.addWidget(self.table)
        self.model.dataChanged.connect(self.save_to_json)
        self.type_filter.currentTextChanged.connect(self.filter_table)
        self.load_from_json()
        self.filter_table("All")


    def add_to_table(self):
        _type = f"_{self.type_filter.currentText()}"
        # print(_type)
        notes = self.note_input.toPlainText()
        idea = self.idea_input.toPlainText()
        importance = self.importance_input.value()
        complexity = self.complexity_input.value()
        time = self.time_input.value()
        productivity_score = (importance)/(complexity*time)

        type_item = QStandardItem()
        type_item.setData(_type, Qt.DisplayRole)

        idea_item = QStandardItem()
        idea_item.setData(idea, Qt.DisplayRole)

        importance_item = QStandardItem()
        importance_item.setData(importance, Qt.DisplayRole)

        complexity_item = QStandardItem()
        complexity_item.setData(complexity, Qt.DisplayRole)

        time_item = QStandardItem()
        time_item.setData(time, Qt.DisplayRole)

        productivity_score_item = QStandardItem()
        productivity_score_item.setData(productivity_score, Qt.DisplayRole)

        notes_item = QStandardItem()
        notes_item.setData(notes, Qt.DisplayRole)

        self.model.appendRow([
            type_item,
            idea_item,
            importance_item,
            complexity_item,
            time_item,
            productivity_score_item,
            notes_item
        ])

        # Clear the inputs
        self.idea_input.clear()
        self.importance_input.setValue(1)
        self.complexity_input.setValue(1)
        self.time_input.setValue(1)
        self.save_to_json()
    
    def save_to_json(self):
        data = []
        previous_choice = self.type_filter.currentText()
        self.filter_table("All")
        for i in range(self.model.rowCount()):
            row = {}
            for j in range(self.model.columnCount()):
                index = self.model.index(i, j)
                if index.isValid():
                    row[self.model.horizontalHeaderItem(j).text()] = self.model.data(index)

            data.append(row)

        with open(self.filename, 'w') as f:
            json.dump(data, f)
        
        self.filter_table(previous_choice)


    def load_from_json(self):
        try:
            with open(self.filename, 'r') as f:
                data = json.load(f)

            for row_data in data:
                row = []
                for header in ['_Type', 'Idea', 'Importance', 'Complexity', 'Time Consumption', 'Productivity Score', 'Notes']:
                    item = QStandardItem()
                    item.setData(row_data.get(header, ""), Qt.DisplayRole)
                    row.append(item)

                self.model.appendRow(row)
        except FileNotFoundError:
            pass  # Datei existiert nicht, nichts zu tun

    def filter_table(self, _type):
        if _type == "All":
            for row in range(self.model.rowCount()):
                self.table.setRowHidden(row, False)
        else:
            for row in range(self.model.rowCount()):
                if f"{self.model.item(row, 0).text()}"[1:] == _type:
                    self.table.setRowHidden(row, False)
                else:
                    if self.hide_rows:
                        self.table.setRowHidden(row, True)

        for col in range(self.model.columnCount()):
            val = True if self.hide_cols else False
            column_name = self.model.headerData(col, Qt.Horizontal)
            if column_name[0] == "_":
                self.table.setColumnHidden(col, val)

    # def select_row(self, index):
    #     self.selected_row = index.row()
    #     if self.selected_row is not None:
    #         # Get the data from the selected row
    #         _type = self.model.item(self.selected_row, 0).data(Qt.DisplayRole)
    #         idea = self.model.item(self.selected_row, 1).data(Qt.DisplayRole)
    #         importance = self.model.item(self.selected_row, 2).data(Qt.DisplayRole)
    #         complexity = self.model.item(self.selected_row, 3).data(Qt.DisplayRole)
    #         time = self.model.item(self.selected_row, 4).data(Qt.DisplayRole)
    #         notes = self.model.item(self.selected_row, 6).data(Qt.DisplayRole)

    #         # Set the values to the controls
    #         self.type_filter.setCurrentText(_type[1:])
    #         self.idea_input.setPlainText(idea)
    #         self.importance_input.setValue(int(importance))
    #         self.complexity_input.setValue(int(complexity))
    #         self.time_input.setValue(int(time))
    #         self.note_input.setPlainText(notes)
    def select_row(self, index):
        self.selected_row = index.row()
        if self.selected_row is not None:
            # Get the data from the selected row
            _type = self.model.item(self.selected_row, 0).data(Qt.DisplayRole)
            idea = self.model.item(self.selected_row, 1).data(Qt.DisplayRole)
            importance = self.model.item(self.selected_row, 2).data(Qt.DisplayRole)
            complexity = self.model.item(self.selected_row, 3).data(Qt.DisplayRole)
            time = self.model.item(self.selected_row, 4).data(Qt.DisplayRole)
            notes = self.model.item(self.selected_row, 6).data(Qt.DisplayRole)

            # Set the values to the controls
            self.type_filter.setCurrentText(_type[1:])
            self.idea_input.setPlainText(idea)
            self.importance_input.setValue(int(importance))
            self.complexity_input.setValue(int(complexity))
            self.time_input.setValue(int(time))
            self.note_input.setPlainText(notes)

            # Recalculate productivity_score
            productivity_score = (int(importance)) / (int(complexity) * int(time))
            self.model.item(self.selected_row, 5).setData(productivity_score, Qt.DisplayRole)
            self.save_to_json()

    def delete_row(self):
        if self.selected_row is not None:
            # Before deletion, save the data of the row to be deleted
            row_data = []
            for col in range(self.model.columnCount()):
                row_data.append(self.model.item(self.selected_row, col).data(Qt.DisplayRole))
            # Push the data of the deleted row to the stack
            self.deleted_rows.append(row_data)
            
            self.model.removeRow(self.selected_row)
            self.save_to_json()
            self.selected_row = None  # Reset selected row

    def undo_last_deletion(self):
        if self.deleted_rows: # If there are any deleted rows
            row_data = self.deleted_rows.pop() # Pop the last deleted row data
            row = [QStandardItem(str(i)) for i in row_data]
            self.model.appendRow(row) # Add the row data back to the table
            self.save_to_json()

if __name__ == "__main__":
    app = QApplication([])
    font = QFont()
    font.setPointSize(16)  # Set the initial font size
    app.setFont(font)
    app.setApplicationName("Sample Explorer")
    app.setApplicationDisplayName("Sample Explorer")
    window = MainWindow()
    window.show()
    app.exec()
