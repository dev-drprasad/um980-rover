import serial
import socket

# --- Configuration ---
# Change this to match your setup ('COM3' on Windows, '/dev/ttyUSB0' or '/dev/ttyACM0' on Linux/Mac)
SERIAL_PORT = '/dev/ttyUSB0' 
BAUD_RATE = 115200

TCP_IP = '0.0.0.0'  # Listen on all network interfaces
TCP_PORT = 5005     # The port for the TCP server

def main():
    # 1. Open the Serial Port
    print(f"Opening serial port {SERIAL_PORT} at {BAUD_RATE} baud...")
    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1)
    except Exception as e:
        print(f"Failed to open serial port: {e}")
        return

    # 2. Setup the TCP Server
    print(f"Starting TCP server on port {TCP_PORT}...")
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    
    # This option prevents the "Address already in use" error if you restart the script quickly
    server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1) 
    server_socket.bind((TCP_IP, TCP_PORT))
    server_socket.listen(1)

    print("Waiting for a TCP client to connect...")
    conn, addr = server_socket.accept()
    print(f"Client connected from: {addr}")

    # 3. Main Loop: Read Serial -> Send TCP
    print("Forwarding data. Press Ctrl+C to stop.")
    try:
        while True:
            # Check if there is data waiting in the serial buffer
            if ser.in_waiting > 0:
                # Read all available bytes
                data = ser.read(ser.in_waiting) 
                
                # Send the bytes over the TCP connection
                conn.sendall(data) 
                
    except KeyboardInterrupt:
        print("\nStopping script...")
    except Exception as e:
        print(f"\nConnection error: {e}")
    finally:
        # 4. Clean up resources
        conn.close()
        server_socket.close()
        ser.close()
        print("Connections closed.")

if __name__ == '__main__':
    main()
