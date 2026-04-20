import { Canvas } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import { useRef, useEffect } from "react";
import * as THREE from "three";

interface WiimoteData {
  buttons: string[];
  acc_x: number;
  acc_y: number;
  acc_z: number;
  battery: number;
}

interface VisualizerProps {
  wiimoteData: WiimoteData;
}

function ColoredBox({ wiimoteData }: VisualizerProps) {
  const meshRef = useRef<THREE.Group>(null);

  useEffect(() => {
    if (meshRef.current) {
      // Calculate rotation from accelerometer data
      // When Wiimote is flat on table: acc_x ≈ 0, acc_y ≈ 0, acc_z ≈ 1g
      // We want Z-axis pointing up (vertically), so:
      // - Use atan2 to get roll and pitch from accelerometer
      
      const roll = Math.atan2(wiimoteData.acc_y, wiimoteData.acc_z);
      const pitch = Math.atan2(-wiimoteData.acc_x, 
                               Math.sqrt(wiimoteData.acc_y ** 2 + wiimoteData.acc_z ** 2));

      meshRef.current.rotation.x = -roll;
      meshRef.current.rotation.z = pitch;
      meshRef.current.rotation.y = 0; // No yaw from accelerometer alone
    }
  }, [wiimoteData]);

  return (
    <group ref={meshRef}>
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
