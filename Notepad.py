from PySide6.QtWidgets import QApplication, QLabel, QVBoxLayout, QHBoxLayout, QWidget, QTextEdit, QTableView, QPushButton, QSpinBox, QComboBox,QHeaderView, QStyledItemDelegate
from PySide6.QtGui import QStandardItemModel, QStandardItem, QFont, QFontMetrics
from PySide6.QtCore import Qt
import json
import os

dev=False
    
class MainWindow(QWidget):
    def __init__(self):
        super().__init__()
        self.printout=False
        self.hide_rows, self.hide_cols = True, True
        self.selected_row, self.count = None, 0
        self.deleted_rows = []
        self.sort_order = None
        self.sorted_column_index = None
        self.selected_row_state = None
        file_path = os.path.realpath(__file__)
        folder_path = os.path.dirname(file_path)
        self.filename = os.path.join(folder_path, r'notes.json')
        self.layout = QVBoxLayout(self)

        # Add the type_filter ComboBox
        self.type_filter = QComboBox()
        delegate = AlignDelegate(self.type_filter)
        self.type_filter.setItemDelegate(delegate)
        self.type_filter.addItems(["All", "Ideas", "Hypotheses", "Experiments"])
        self.type_filter.currentTextChanged.connect(self.filter_table)
        # Set the stylesheet to add padding to the text
        self.type_filter.currentTextChanged.connect(self.center_text_in_combobox)

        self.layout.addWidget(self.type_filter)

        # Create QHBoxLayouts for each TextEdit and their labels
        idea_layout = QVBoxLayout()
        idea_layout.addWidget(QLabel("Idea"))
        self.idea_input = QTextEdit()
        idea_layout.addWidget(self.idea_input)

        notes_layout = QVBoxLayout()
        notes_layout.addWidget(QLabel("Notes"))
        self.note_input = QTextEdit()
        notes_layout.addWidget(self.note_input)

        # Create a QHBoxLayout to contain the two QVBoxLayouts
        textedit_layout = QHBoxLayout()
        textedit_layout.addLayout(idea_layout)
        textedit_layout.addLayout(notes_layout)
        self.layout.addLayout(textedit_layout)

        # Create a QHBoxLayout for SpinBoxes
        spinbox_layout = QHBoxLayout()
        for label in ["Importance", "Complexity", "Time Consumption"]:
            spinbox_layout.addWidget(QLabel(label))
            spinbox = QSpinBox()
            spinbox.setAlignment(Qt.AlignCenter)
            spinbox.setRange(1, 6)
            labell=label.lower().replace(" ","_")
            setattr(self, f"{labell}_input", spinbox)
            spinbox_layout.addWidget(spinbox)
        self.layout.addLayout(spinbox_layout)

        # Create a QHBoxLayout for buttons
        button_layout = QHBoxLayout()
        self.add_button = QPushButton(f"Add")
        self.add_button.clicked.connect(self.add_to_table)
        button_layout.addWidget(self.add_button)

        self.save_changes_button = QPushButton("Save Changes")
        self.save_changes_button.clicked.connect(self.save_changes)
        button_layout.addWidget(self.save_changes_button)

        self.delete_button = QPushButton("Delete Entry")
        self.delete_button.clicked.connect(self.delete_row)
        button_layout.addWidget(self.delete_button)

        self.undo_button = QPushButton("Undo Last Deletion")
        self.undo_button.clicked.connect(self.undo_last_deletion)
        button_layout.addWidget(self.undo_button)
        self.layout.addLayout(button_layout)

        self.model = QStandardItemModel(0, 6)
        self.model.setHorizontalHeaderLabels(['_Type','Idea', 'Importance', 'Complexity', 'Time Consumption', 'Productivity Score', 'Notes'])
       
        self.table = QTableView()
        self.table.verticalHeader().setVisible(False)
        self.table.horizontalHeader().setSectionResizeMode(QHeaderView.ResizeToContents)
        self.table.setSortingEnabled(True)
        self.table.clicked.connect(self.select_row)
        self.table.setModel(self.model)
        self.table.setAlternatingRowColors(True) # Add this line
        self.table.setStyleSheet("alternate-background-color: #BBFF9E; background-color: white;")
        self.setAlignmentCenter(['Importance', 'Complexity', 'Time Consumption', 'Productivity Score'])
        self.layout.addWidget(self.table)

        self.model.dataChanged.connect(self.save_to_json)

        self.load_from_json()
        self.filter_table("All")

        for label in self.findChildren(QLabel):
            label.setAlignment(Qt.AlignCenter)

        #Check ValueChanged when Editing exisiting Rows
        self.save_changes_button.setEnabled(False)
        [getattr(getattr(self, f"{control}_input"), signal).connect(self.check_changes) for control in ['idea', 'note', 'importance', 'complexity', 'time_consumption'] for signal in ['textChanged' if control in ['idea', 'note'] else 'valueChanged']]



    #Override ResizeEvent
    def resizeEvent(self, event):
        super(MainWindow, self).resizeEvent(event)
        self.center_text_in_combobox(self.type_filter.currentText())

    def setAlignmentCenter(self, center_columns):
        for col in range(self.model.columnCount()):
            column_name = self.model.headerData(col, Qt.Horizontal)
            if column_name in center_columns:
                self.table.setColumnWidth(col, 150)  # Adjust column width if necessary
                self.table.setItemDelegateForColumn(col, AlignDelegate(self.table))  # Apply delegate for center alignment

    def center_text_in_combobox(self, text):
        metrics = QFontMetrics(QApplication.font())
        text_width = metrics.horizontalAdvance(text)
        combobox_width = self.type_filter.width()
        padding = (combobox_width - text_width) / 2
        stylesheet = f"QComboBox {{ padding-left: {padding}px; }}"
        self.type_filter.setStyleSheet(stylesheet)

    def p(self, text):
        if self.printout:
            print(text)

    def add_to_table(self):
        _type = f"_{self.type_filter.currentText()}"
        # print(_type)
        notes = self.note_input.toPlainText()
        idea = self.idea_input.toPlainText()
        importance = self.importance_input.value()
        complexity = self.complexity_input.value()
        time = self.time_consumption_input.value()
        productivity_score = (importance**2)/(complexity*time)

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
        self.note_input.clear()
        self.importance_input.setValue(1)
        self.complexity_input.setValue(1)
        self.time_consumption_input.setValue(1)
        self.save_to_json()

    
    def check_order(self):
        self.sort_order = self.table.horizontalHeader().sortIndicatorOrder()
        self.sorted_column_index = self.table.horizontalHeader().sortIndicatorSection()
        self.table.sortByColumn(self.sorted_column_index, self.sort_order )

    def check_changes(self):
        # Current state
        _type = f"_{self.type_filter.currentText()}"
        idea = self.idea_input.toPlainText()
        importance = self.importance_input.value()
        complexity = self.complexity_input.value()
        time = self.time_consumption_input.value()
        notes = self.note_input.toPlainText()

        current_state = [_type, idea, importance, complexity, time, notes]

        # Check if the current state matches the saved state
        if current_state != self.selected_row_state:
            self.save_changes_button.setEnabled(True)
        else:
            self.save_changes_button.setEnabled(False)

    def save_changes(self):
        if self.selected_row is not None:
            _type = f"_{self.type_filter.currentText()}"
            idea = self.idea_input.toPlainText()
            importance = self.importance_input.value()
            complexity = self.complexity_input.value()
            time = self.time_consumption_input.value()
            notes = self.note_input.toPlainText()
            productivity_score = (importance**2) / (complexity * time)

            self.model.item(self.selected_row, 0).setData(_type, Qt.DisplayRole)
            self.model.item(self.selected_row, 1).setData(idea, Qt.DisplayRole)
            self.model.item(self.selected_row, 2).setData(importance, Qt.DisplayRole)
            self.model.item(self.selected_row, 3).setData(complexity, Qt.DisplayRole)
            self.model.item(self.selected_row, 4).setData(time, Qt.DisplayRole)
            self.model.item(self.selected_row, 5).setData(productivity_score, Qt.DisplayRole)
            self.model.item(self.selected_row, 6).setData(notes, Qt.DisplayRole)
            self.selected_row_state = [_type, idea, importance, complexity, time, notes]
            self.save_changes_button.setEnabled(False)
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
        self.check_order()

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
        self.count=0
        if _type == "All":
            for row in range(self.model.rowCount()):
                self.count+=1
                self.table.setRowHidden(row, False)
        else:
            for row in range(self.model.rowCount()):
                if f"{self.model.item(row, 0).text()}"[1:] == _type:
                    self.table.setRowHidden(row, False)
                    self.count+=1

                else:
                    if self.hide_rows:
                        self.table.setRowHidden(row, True)

        for col in range(self.model.columnCount()):
            val = True if self.hide_cols else False
            column_name = self.model.headerData(col, Qt.Horizontal)
            if column_name[0] == "_":
                self.table.setColumnHidden(col, val)
        self.add_button.setText(f"Add Entry (Total: {self.count+1})")

    def select_row(self, index):
        self.selected_row = index.row()
        self.p(f"Selected Row {self.selected_row}")
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
            self.time_consumption_input.setValue(int(time))
            self.note_input.setPlainText(notes)

            # Recalculate productivity_score
            productivity_score = (int(importance)**2) / (int(complexity) * int(time))
            self.model.item(self.selected_row, 5).setData(productivity_score, Qt.DisplayRole)
            if dev:
                self.save_changes_button.setText(f"Save Changes in #{self.selected_row}")
            else:
                self.save_changes_button.setText(f"Save current Changes")
            
            #Handle the Save Changes Button
            self.save_changes_button.setEnabled(False)
            self.selected_row_state = [_type, idea, importance, complexity, time, notes]
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

#Helper Class to Center Text in QCombobox            
class AlignDelegate(QStyledItemDelegate):
    def initStyleOption(self, option, index):
        super(AlignDelegate, self).initStyleOption(option, index)
        option.displayAlignment = Qt.AlignCenter

if __name__ == "__main__":
    app = QApplication([])
    font = QFont()
    font.setPointSize(16)  # Set the initial font size
    app.setFont(font)
    app.setApplicationName("Idea Notepad")
    app.setApplicationDisplayName("Idea Notepad")
    window = MainWindow()
    window.printout=True
    window.showMaximized()
    app.exec()
