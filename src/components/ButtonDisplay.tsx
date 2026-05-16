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

interface ButtonDisplayProps {
  wiimoteData: WiimoteData;
}

// Quaternion to Euler angles (ZYX order) - returns degrees
// This order minimizes gimbal lock artifacts for Yaw rotation
function quaternionToEuler(qw: number, qx: number, qy: number, qz: number): { roll: number; pitch: number; yaw: number } {
  // ZYX order (intrinsic rotations)
  // Roll (X-axis): atan2
  // Pitch (Y-axis): asin (gimbal lock axis)
  // Yaw (Z-axis): atan2
  
  const sinr = 2 * (qw * qx - qz * qy);
  const cosr = 1 - 2 * (qx * qx + qy * qy);
  const roll = Math.atan2(sinr, cosr);

  const sinp = 2 * (qw * qy + qz * qx);
  const pitch = Math.asin(Math.max(-1, Math.min(1, sinp)));

  const siny = 2 * (qw * qz - qx * qy);
  const cosy = 1 - 2 * (qy * qy + qz * qz);
  const yaw = Math.atan2(siny, cosy);

  return {
    roll: roll * (180 / Math.PI),
    pitch: pitch * (180 / Math.PI),
    yaw: yaw * (180 / Math.PI),
  };
}

const allButtons = ["A", "B", "1", "2", "+", "-", "HOME", "UP", "DOWN", "LEFT", "RIGHT"];

export default function ButtonDisplay({ wiimoteData }: ButtonDisplayProps) {
  const isPressed = (buttonName: string) => wiimoteData.buttons.includes(buttonName);

  // Calculate Euler angles from quaternion
  const euler = wiimoteData.quat_valid
    ? quaternionToEuler(wiimoteData.quat_w, wiimoteData.quat_x, wiimoteData.quat_y, wiimoteData.quat_z)
    : { roll: 0, pitch: 0, yaw: 0 };

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
        <div className="info-column">
          <div className="info-column-title">Tilt </div>
          <div className="info-item">
            <span className="label">X:</span>
            <span className="value">{wiimoteData.acc_x.toFixed(2)}g</span>
          </div>
          <div className="info-item">
            <span className="label">Z:</span>
            <span className="value">{wiimoteData.acc_z.toFixed(2)}g</span>
          </div>
          <div className="info-item">
            <span className="label">Y:</span>
            <span className="value">{wiimoteData.acc_y.toFixed(2)}g</span>
          </div>
        </div>

        <div className="info-column">
          <div className="info-column-title">Orientation</div>
          <div className="info-item">
            <span className="label">Roll (X):</span>
            <span className="value">{euler.roll.toFixed(1)}</span>
          </div>
          <div className="info-item">
            <span className="label">Pitch (Y):</span>
            <span className="value">{euler.pitch.toFixed(1)}</span>
          </div>
          <div className="info-item">
            <span className="label">Yaw (Z):</span>
            <span className="value">{euler.yaw.toFixed(1)}</span>
          </div>
          <div className="info-item">
            <span className="label">Status:</span>
            <span className={`value ${wiimoteData.motion_plus_active ? "ok" : "warn"}`}>
              {wiimoteData.motion_plus_active ? "GYRO" : "ACCEL"}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

