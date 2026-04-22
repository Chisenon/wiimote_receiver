import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
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

  return (
    <div className="app">
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
