import { Canvas, useFrame } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import { useRef, useEffect } from "react";
import * as THREE from "three";

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

interface VisualizerProps {
  wiimoteData: WiimoteData;
}

function ColoredBox({ wiimoteData }: VisualizerProps) {
  const meshRef = useRef<THREE.Group>(null);
  const targetQuaternionRef = useRef(new THREE.Quaternion(0, 0, 0, 1));

  useEffect(() => {
    // Quaternion を使用して正確な回転を適用
    // lib.rs から送信された quaternion: quat_w, quat_x, quat_y, quat_z
    if (wiimoteData.quat_valid) {
      const q = new THREE.Quaternion(
        wiimoteData.quat_x,
        wiimoteData.quat_y,
        wiimoteData.quat_z,
        wiimoteData.quat_w
      );
      targetQuaternionRef.current.copy(q);
    }
  }, [wiimoteData]);

  useFrame(() => {
    if (!meshRef.current) {
      return;
    }
    // Quaternion を smooth に補間
    const currentQuat = meshRef.current.quaternion;
    currentQuat.slerp(targetQuaternionRef.current, 0.15);
  });

  return (
    <group ref={meshRef}>
      {/* 
        軸定義（Wiimote フレーム）:
        - X軸 (赤): 左右（横方向）
        - Y軸 (緑): 前後（奥行き方向）
        - Z軸 (青): 上下（縦方向）
        
        Quaternion は lib.rs の complementary filter で計算され、
        加速度計データとジャイロデータを融合している。
        これにより gimbal lock を回避し、正確な3D回転を実現。
      */}
      
      {/* Z軸 (青) - 上下の面 */}
      {/* 上面 - 明るい青 */}
      <mesh position={[0, 0, 2]}>
        <planeGeometry args={[2.4, 2.4]} />
        <meshPhongMaterial color="#6666ff" emissive="#0000ff" side={THREE.DoubleSide} />
      </mesh>
      {/* 下面 - 暗い青 */}
      <mesh position={[0, 0, -2]} rotation={[0, 0, 0]}>
        <planeGeometry args={[2.4, 2.4]} />
        <meshPhongMaterial color="#0000cc" emissive="#000099" side={THREE.DoubleSide} />
      </mesh>

      {/* X軸 (赤) - 左右の面 */}
      {/* 右面 - 明るい赤 */}
      <mesh position={[1.2, 0, 0]} rotation={[0, Math.PI / 2, 0]}>
        <planeGeometry args={[4, 2.4]} />
        <meshPhongMaterial color="#ff6666" emissive="#ff0000" side={THREE.DoubleSide} />
      </mesh>
      {/* 左面 - 暗い赤 */}
      <mesh position={[-1.2, 0, 0]} rotation={[0, Math.PI / 2, 0]}>
        <planeGeometry args={[4, 2.4]} />
        <meshPhongMaterial color="#cc0000" emissive="#990000" side={THREE.DoubleSide} />
      </mesh>

      {/* Y軸 (緑) - 前後の面 */}
      {/* 前面 - 明るい緑 */}
      <mesh position={[0, 1.2, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <planeGeometry args={[2.4, 4]} />
        <meshPhongMaterial color="#66ff66" emissive="#00ff00" side={THREE.DoubleSide} />
      </mesh>
      {/* 背面 - 暗い緑 */}
      <mesh position={[0, -1.2, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <planeGeometry args={[2.4, 4]} />
        <meshPhongMaterial color="#00cc00" emissive="#009900" side={THREE.DoubleSide} />
      </mesh>

      {/* 枠線用ワイヤーフレーム */}
      <mesh>
        <boxGeometry args={[2.4, 2.4, 4]} />
        <meshPhongMaterial wireframe color="#333" transparent opacity={0.2} />
      </mesh>
    </group>
  );
}

export default function WiimoteVisualizer({ wiimoteData }: VisualizerProps) {
  return (
    <div className="visualizer">
      <Canvas camera={{ position: [3, 3, 3] }}>
        <ambientLight intensity={0.6} />
        <pointLight position={[10, 10, 10]} intensity={0.8} />
        <pointLight position={[-10, -10, 10]} intensity={0.4} />
        <ColoredBox wiimoteData={wiimoteData} />
        <OrbitControls />
      </Canvas>
    </div>
  );
}
