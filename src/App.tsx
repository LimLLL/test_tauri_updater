import {useLayoutEffect, useState} from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import checkForAppUpdates from "./updater.tsx";
import {getVersion} from "@tauri-apps/api/app";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [version, setVersion] = useState("");
  const [downloaded, setDownloaded] = useState<number>(0);
  const [contentLength, setContentLength] = useState<number>(0);

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  useLayoutEffect(() => {
    checkForAppUpdates(setDownloaded, setContentLength).then(r => console.log(r));
    getVersion().then(v => setVersion(v));
  }, [])

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet {version}</button>
        <p>
          {Math.round((downloaded / contentLength) * 100)}% downloaded
        </p>
      </form>
      <p>{greetMsg}</p>
    </main>
  );
}

export default App;
