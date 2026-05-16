import { useState, useEffect, useRef, type ChangeEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import WiimoteVisualizer from "./components/WiimoteVisualizer";
import ButtonDisplay from "./components/ButtonDisplay";

interface WiimoteData {
  buttons: string[];
  acc_x: number;
  acc_y: number;
  acc_z: number;
  gyro_yaw: number;
  gyro_roll: number;
  gyro_pitch: number;
  quat_w: number;
  quat_x: number;
  quat_y: number;
  quat_z: number;
  quat_valid: boolean;
  motion_plus_active: boolean;
}

function App() {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [configStatus, setConfigStatus] = useState<string>("config: ready");
  const [configPath, setConfigPath] = useState<string>("");
  const [wiimoteData, setWiimoteData] = useState<WiimoteData>({
    buttons: [],
    acc_x: 0,
    acc_y: 0,
    acc_z: 0,
    gyro_yaw: 0,
    gyro_roll: 0,
    gyro_pitch: 0,
    quat_w: 1,
    quat_x: 0,
    quat_y: 0,
    quat_z: 0,
    quat_valid: false,
    motion_plus_active: false,
  });

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setupListener = async () => {
      try {
        unlisten = await listen<WiimoteData>("wiimote-data", (event) => {
          setWiimoteData(event.payload);
        });
        console.log("Wiimote listener started");
      } catch (error) {
        console.error("Failed to setup listener:", error);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    const setupConfigPath = async () => {
      try {
        const path = await invoke<string>("get_config_path");
        setConfigPath(path);
      } catch (error) {
        console.error("Failed to get config path:", error);
      }
    };
    setupConfigPath();
  }, []);

  const handleCreateConfig = async () => {
    setConfigStatus("config: creating...");
    try {
      const path = await invoke<string>("ensure_config_file");
      setConfigPath(path);
      setConfigStatus("config: file ready");
    } catch (error) {
      console.error(error);
      setConfigStatus("config: create failed");
    }
  };

  const handlePickConfig = () => {
    fileInputRef.current?.click();
  };

  const handleLoadConfigFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setConfigStatus(`config: loading ${file.name}...`);
    try {
      const json = await file.text();
      await invoke<string>("import_config_from_json", { json });
      const path = await invoke<string>("get_config_path");
      setConfigPath(path);
      setConfigStatus("config: loaded and applied");
    } catch (error) {
      console.error(error);
      setConfigStatus("config: load failed");
    } finally {
      event.target.value = "";
    }
  };

  return (
    <div className="app">
      <div className="hotbar">
        <div className="hotbar-left">
          <button className="hotbar-btn" onClick={handlePickConfig}>
            Load Config
          </button>
          <button className="hotbar-btn secondary" onClick={handleCreateConfig}>
            Create Config
          </button>
          <span className="hotbar-status">{configStatus}</span>
        </div>
        <div className="hotbar-path">{configPath}</div>
        <input
          ref={fileInputRef}
          type="file"
          accept=".json,application/json"
          className="hidden-file-input"
          onChange={handleLoadConfigFile}
        />
      </div>
      <div className="visualizer-container">
        <WiimoteVisualizer wiimoteData={wiimoteData} />
      </div>
      <div className="button-container">
        <ButtonDisplay wiimoteData={wiimoteData} />
      </div>
    </div>
  );
}

export default App;
