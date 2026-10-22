// Omarchy's login theme with a session picker added (F2 or click to cycle), so Slipstream can be
// chosen. Installed by Slipstream's extras/omarchy-switch/install, and removed by its --uninstall
// (or removed by uninstall-slipstream).
import QtQuick 2.0
import SddmComponents 2.0

Rectangle {
  id: root
  width: 640
  height: 480
  color: "#1a1b26"

  property string currentUser: userModel.lastUser
  property bool loginFailed: false
  property int sessionIndex: sessionModel.lastIndex

  // sessionModel only answers by role name, so a hidden Repeater reads each session's name.
  Repeater {
    id: sessions
    model: sessionModel
    delegate: Item { property string sessionName: model.name || "" }
  }

  function sessionName(i) {
    var item = sessions.itemAt(i)
    return item ? item.sessionName : ""
  }

  // Starts on Slipstream, whatever was used last; without it, on Omarchy (else any uwsm session).
  // F2 still cycles to the others, so a broken Slipstream is one key away from a working desktop.
  function pickDefault() {
    var omarchy = -1
    var uwsm = -1
    for (var i = 0; i < sessions.count; i++) {
      var name = sessionName(i)
      if (name === "Slipstream") {
        root.sessionIndex = i
        return
      }
      if (omarchy < 0 && name.indexOf("Omarchy") !== -1)
        omarchy = i
      if (uwsm < 0 && name.indexOf("uwsm") !== -1)
        uwsm = i
    }
    if (omarchy >= 0)
      root.sessionIndex = omarchy
    else if (uwsm >= 0)
      root.sessionIndex = uwsm
  }

  function nextSession() {
    if (sessions.count > 0)
      root.sessionIndex = (root.sessionIndex + 1) % sessions.count
  }

  Connections {
    target: sddm
    function onLoginFailed() {
      root.loginFailed = true
      password.text = ""
      password.focus = true
    }
    function onLoginSucceeded() {
      root.loginFailed = false
    }
  }

  Column {
    anchors.centerIn: parent
    spacing: 40

    Image {
      id: logo
      source: "logo.png"
      width: Math.min(sourceSize.width, root.width * 0.8)
      height: sourceSize.width > 0 ? Math.round(width * sourceSize.height / sourceSize.width) : 0
      fillMode: Image.PreserveAspectFit
      anchors.horizontalCenter: parent.horizontalCenter
    }

    Row {
      anchors.horizontalCenter: parent.horizontalCenter
      spacing: 15

      Image {
        source: root.loginFailed ? "lock-failed.png" : "lock.png"
        width: 34
        height: 38
        fillMode: Image.PreserveAspectFit
        anchors.verticalCenter: parent.verticalCenter
      }

      Item {
        width: entry.width
        height: entry.height

        Image {
          id: entry
          source: root.loginFailed ? "entry-failed.png" : "entry.png"
          anchors.centerIn: parent
        }

        Row {
          anchors.left: parent.left
          anchors.leftMargin: 20
          anchors.verticalCenter: parent.verticalCenter
          spacing: 5

          Repeater {
            model: Math.min(password.text.length, 21)

            Image {
              source: "bullet.png"
              width: 7
              height: 7
            }
          }
        }

        TextInput {
          id: password
          anchors.fill: parent
          anchors.leftMargin: 20
          anchors.rightMargin: 20
          verticalAlignment: TextInput.AlignVCenter
          echoMode: TextInput.Password
          font.family: "JetBrainsMono Nerd Font"
          font.pixelSize: 24
          font.letterSpacing: 5
          passwordCharacter: "•"
          color: "transparent"
          selectionColor: "transparent"
          selectedTextColor: "transparent"
          cursorDelegate: Item {}
          focus: true

          onTextChanged: root.loginFailed = false

          Keys.onPressed: {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
              sddm.login(root.currentUser, password.text, root.sessionIndex)
              event.accepted = true
            } else if (event.key === Qt.Key_F2) {
              root.nextSession()
              event.accepted = true
            }
          }
        }
      }
    }

    Text {
      anchors.horizontalCenter: parent.horizontalCenter
      text: sessions.count > 0 ? "‹ " + root.sessionName(root.sessionIndex) + " ›   F2" : ""
      font.family: "JetBrainsMono Nerd Font"
      font.pixelSize: 20
      color: "#737aa2"

      MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: {
          root.nextSession()
          password.forceActiveFocus()
        }
      }
    }
  }

  Component.onCompleted: {
    pickDefault()
    password.forceActiveFocus()
  }
}
