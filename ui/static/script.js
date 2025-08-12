function updateStatus() {
    const status = document.getElementById("status");
    status.textContent = "Stato: aggiornato alle " + new Date().toLocaleTimeString();
}
