interface WiimoteData {
  buttons: string[];
  acc_x: number;
  acc_y: number;
  acc_z: number;
}

interface ButtonDisplayProps {
  wiimoteData: WiimoteData;
}

const allButtons = ["A", "B", "1", "2", "+", "-", "HOME", "↑", "↓", "←", "→"];

export default function ButtonDisplay({ wiimoteData }: ButtonDisplayProps) {
  const isPressed = (buttonName: string) => wiimoteData.buttons.includes(buttonName);

  return (
    <div className="button-display">
      <div className="button-grid">
        {allButtons.map((button) => (
          <div
            key={button}
            className={`button-item ${isPressed(button) ? "pressed" : ""}`}
          >
            <span className="button-label">{button}</span>
          </div>
        ))}
      </div>
      <div className="info-panel">
        <div className="info-item">
          <span className="label">Acceleration X (赤):</span>
          <span className="value">{wiimoteData.acc_x.toFixed(2)}g</span>
        </div>
        <div className="info-item">
          <span className="label">Acceleration Z (緑):</span>
          <span className="value">{wiimoteData.acc_z.toFixed(2)}g</span>
        </div>
        <div className="info-item">
          <span className="label">Acceleration Y (青):</span>
          <span className="value">{wiimoteData.acc_y.toFixed(2)}g</span>
        </div>
      </div>
    </div>
  );
}
