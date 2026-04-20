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
}

function App() {
  const [wiimoteData, setWiimoteData] = useState<WiimoteData>({
    buttons: [],
    acc_x: 0,
    acc_y: 0,
    acc_z: 0,
    battery: 0,
  });

  useEffect(() => {
    let unlistenPromise: Promise<() => void> | null = null;

    const setupListener = async () => {
      try {
        unlistenPromise = await listen<WiimoteData>("wiimote-data", (event) => {
          setWiimoteData(event.payload);
        });
        console.log("Wiimote listener started");
      } catch (error) {
        console.error("Failed to setup listener:", error);
      }
    };

    setupListener();

    return () => {
      if (unlistenPromise) {
        unlistenPromise.then((unlisten) => unlisten());
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
