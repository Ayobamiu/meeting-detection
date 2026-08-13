// Bridges the main process to the UI. The renderer never touches the native
// addon directly — it cannot, and it should not need to.

const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("meetingDetection", {
  onStatus: (callback) =>
    ipcRenderer.on("status", (_event, payload) => callback(payload)),

  onMeetingEvent: (callback) =>
    ipcRenderer.on("meeting-event", (_event, payload) => callback(payload)),

  getDetails: () => ipcRenderer.invoke("get-details"),

  /** Tells main the update was received and rendered. */
  ack: (payload) => ipcRenderer.send("renderer-ack", payload),
});
