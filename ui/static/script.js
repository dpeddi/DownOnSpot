function updateStatus() {
    const status = document.getElementById("status");
    status.textContent = "Stato: aggiornato alle " + new Date().toLocaleTimeString();
}

async function status() {
    const res = await fetch("/api/status");
    const downloads = await res.json();
    const container = document.getElementById("status");
    container.innerHTML = "";

    downloads.forEach(dl => {
        const item = document.createElement("div");
        item.className = "download-item";

        const title = document.createElement("span");
        title.textContent = `🎵 ${dl.title}`;

        const state = document.createElement("span");
        let statusText = "";

        if (dl.state?.Downloading) {
            const [current, total] = dl.state.Downloading;
            const percent = ((current / total) * 100).toFixed(1);
            statusText = `⏬ ${percent}%`;
        } else if (dl.state === "Done") {
            statusText = "✅ Completato";
        } else if (dl.state?.Error) {
            statusText = `❌ Errore: ${dl.state.Error}`;
        } else {
            statusText = `⏳ ${dl.state}`;
        }

        state.textContent = ` — ${statusText}`;
        item.appendChild(title);
        item.appendChild(state);
        container.appendChild(item);
    });
}
