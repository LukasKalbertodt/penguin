"use strict";
// This code was inserted by the 'penguin' library. It's here to enable features
// like browser auto-reloading or showing messages. It does this by
// communicating with the penguin server via a websocket.
//
// Configuration dependent values that are passed/interpolated by the penguin
// server.
const control_path = "{{ control_path }}";
// The target URI of the websocket connection.
const wsUri = (() => {
    const scheme = window.location.protocol === "https" ? "wss" : "ws";
    const host = window.location.host;
    return `${scheme}://${host}${control_path}`;
})();
// Open websocket connection and install handlers.
const socket = new WebSocket(wsUri);
socket.addEventListener("close", onConnectionError);
socket.addEventListener("open", () => {
    socket.removeEventListener("close", onConnectionError);
    socket.addEventListener("close", () => console.log("penguin server closed WS connection"));
    socket.addEventListener("message", onMessage);
});
function onConnectionError() {
    console.warn(`Could not connect to web socket backend ${wsUri}`);
}
function onMessage(event) {
    if (typeof event.data !== 'string') {
        throw new Error("unexpected WS message from penguin");
    }
    const endLine = event.data.indexOf('\n');
    const command = event.data.slice(0, endLine === -1 ? undefined : endLine);
    const payload = endLine === -1 ? "" : event.data.slice(endLine + 1);
    switch (command) {
        case "reload":
            console.log("Received reload request from penguin server: reloading page...");
            location.reload();
            break;
        case "message":
            showMessage(payload);
            break;
        default:
            throw new Error("unexpected WS command from penguin");
    }
}
function showMessage(message) {
    let overlay = document.createElement("div");
    let closeButton = document.createElement("button");
    closeButton.innerText = "Close ✖";
    closeButton.style.fontSize = "20px";
    closeButton.style.fontFamily = "sans-serif";
    closeButton.style.display = "inline-block";
    closeButton.style.cursor = "pointer";
    closeButton.addEventListener("click", () => overlay.style.display = "none");
    let header = document.createElement("div");
    header.style.textAlign = "right";
    header.style.margin = "8px";
    header.appendChild(closeButton);
    let content = document.createElement("div");
    content.innerHTML = message;
    content.style.margin = "16px";
    content.style.height = "100%";
    overlay.appendChild(header);
    overlay.appendChild(content);
    overlay.style.position = "fixed";
    overlay.style.zIndex = "987654321"; // Arbitrary very large number
    overlay.style.height = "100vh";
    overlay.style.width = "100vw";
    overlay.style.top = "0";
    overlay.style.left = "0";
    overlay.style.backgroundColor = "#ebebeb";
    document.body.prepend(overlay);
}
