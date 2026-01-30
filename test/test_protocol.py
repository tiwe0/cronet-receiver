#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Cronet Protocol 测试脚本

协议格式:
MagicNumber(4B) | TagPackageLen(2B) | TagUrlLen(2B) | PayloadLen(4B) | 
EndFlag(4B) | TagPackage(M bytes) | TagUrl(N bytes) | Payload(K bytes)
"""

import socket
import struct
import json
import argparse


class ProtocolClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 9000, magic_key: str = "MAGIC"):
        """
        初始化协议客户端
        
        Args:
            host: 服务器地址
            port: 服务器端口
            magic_key: 魔数 (4字节)
        """
        self.host = host
        self.port = port
        self.magic_key = magic_key.encode()[:4].ljust(4, b'\x00')  # 确保是4字节
        
    def create_packet(self, tag_app: str, tag_url: str, payload: bytes, end_flag: bool = False) -> bytes:
        """
        创建协议数据包
        
        Args:
            tag_app: 应用标签
            tag_url: URL标签
            payload: 有效载荷
            end_flag: 结束标志
            
        Returns:
            bytes: 完整的数据包
        """
        tag_app_bytes = tag_app.encode('utf-8')
        tag_url_bytes = tag_url.encode('utf-8')
        
        tag_app_len = len(tag_app_bytes)
        tag_url_len = len(tag_url_bytes)
        payload_len = len(payload)
        end_flag_val = 1 if end_flag else 0
        
        # 构造头部: MagicNumber(4B) | TagAppLen(2B) | TagUrlLen(2B) | PayloadLen(4B) | EndFlag(4B)
        header = struct.pack(
            '>4sHHII',  # > 表示大端序, 4s=4字节, H=2字节无符号, I=4字节无符号
            self.magic_key,
            tag_app_len,
            tag_url_len,
            payload_len,
            end_flag_val
        )
        
        # 组装完整数据包
        packet = header + tag_app_bytes + tag_url_bytes + payload
        
        return packet
    
    def send_packet(self, tag_app: str, tag_url: str, payload: bytes, end_flag: bool = False):
        """
        发送单个数据包
        
        Args:
            tag_app: 应用标签
            tag_url: URL标签
            payload: 有效载荷
            end_flag: 结束标志
        """
        packet = self.create_packet(tag_app, tag_url, payload, end_flag)
        
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.connect((self.host, self.port))
            sock.sendall(packet)
            print(f"[✓] 发送数据包: tag_app={tag_app}, tag_url={tag_url}, payload_size={len(payload)}, end_flag={end_flag}")
    
    def send_chunked_data(self, tag_app: str, tag_url: str, data: bytes, chunk_size: int = 1024):
        """
        分块发送数据
        
        Args:
            tag_app: 应用标签
            tag_url: URL标签
            data: 要发送的数据
            chunk_size: 每块大小
        """
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.connect((self.host, self.port))
            
            total_chunks = (len(data) + chunk_size - 1) // chunk_size
            
            for i in range(0, len(data), chunk_size):
                chunk = data[i:i + chunk_size]
                is_last = (i + chunk_size >= len(data))
                
                packet = self.create_packet(tag_app, tag_url, chunk, end_flag=is_last)
                sock.sendall(packet)
                
                chunk_num = i // chunk_size + 1
                print(f"[✓] 发送分块 {chunk_num}/{total_chunks}: {len(chunk)} bytes, end_flag={is_last}")
    
    def send_json_data(self, tag_app: str, tag_url: str, json_data: dict, chunked: bool = False, chunk_size: int = 1024):
        """
        发送JSON数据
        
        Args:
            tag_app: 应用标签
            tag_url: URL标签
            json_data: JSON数据字典
            chunked: 是否分块发送
            chunk_size: 每块大小(仅当chunked=True时有效)
        """
        payload = json.dumps(json_data, ensure_ascii=False).encode('utf-8')
        
        if chunked:
            print(f"[*] 分块发送JSON数据 (总大小: {len(payload)} bytes, 块大小: {chunk_size} bytes)")
            self.send_chunked_data(tag_app, tag_url, payload, chunk_size)
        else:
            print(f"[*] 发送JSON数据 (大小: {len(payload)} bytes)")
            self.send_packet(tag_app, tag_url, payload, end_flag=True)


def test_simple_message(host: str = "127.0.0.1", port: int = 9000):
    """测试发送简单消息"""
    print("\n=== 测试1: 发送简单JSON消息 ===")
    client = ProtocolClient(host=host, port=port)
    
    data = {
        "message": "Hello, Cronet!",
        "timestamp": "2026-01-30",
        "test_id": 1
    }
    
    client.send_json_data("app1", "/api/test", data)


def test_chunked_message(host: str = "127.0.0.1", port: int = 9000):
    """测试分块发送大消息"""
    print("\n=== 测试2: 分块发送大JSON消息 ===")
    client = ProtocolClient(host=host, port=port)
    
    data = {
        "message": "This is a large message" * 100,
        "items": [{"id": i, "value": f"item_{i}"} for i in range(50)],
        "timestamp": "2026-01-30"
    }
    
    client.send_json_data("app2", "/api/large", data, chunked=True, chunk_size=512)


def test_multiple_connections(host: str = "127.0.0.1", port: int = 9000):
    """测试多个连接"""
    print("\n=== 测试3: 多个连接 ===")
    client = ProtocolClient(host=host, port=port)
    
    for i in range(3):
        data = {
            "connection_id": i,
            "message": f"Message from connection {i}"
        }
        client.send_json_data(f"app{i % 2 + 1}", f"/api/conn/{i}", data)


def test_custom_message(host: str = "127.0.0.1", port: int = 9000):
    """测试自定义消息"""
    print("\n=== 测试4: 自定义消息 ===")
    
    # 获取用户输入
    tag_app = input("请输入 tag_app (默认: app1): ").strip() or "app1"
    tag_url = input("请输入 tag_url (默认: /api/custom): ").strip() or "/api/custom"
    message = input("请输入消息内容 (默认: Custom message): ").strip() or "Custom message"
    
    client = ProtocolClient(host=host, port=port)
    data = {
        "custom_message": message,
        "timestamp": "2026-01-30"
    }
    
    client.send_json_data(tag_app, tag_url, data)


def test_invalid_magic():
    """测试错误的魔数（应该被服务器拒绝）"""
    print("\n=== 测试5: 发送错误魔数（应该失败）===")
    client = ProtocolClient(magic_key="WRONG")
    
    data = {"message": "This should fail"}
    
    try:
        client.send_json_data("app1", "/api/test", data)
    except Exception as e:
        print(f"[!] 预期的错误: {e}")


def main():
    parser = argparse.ArgumentParser(description="Cronet Protocol 测试客户端")
    parser.add_argument("--host", default="127.0.0.1", help="服务器地址 (默认: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=9000, help="服务器端口 (默认: 9000)")
    parser.add_argument("--test", choices=["simple", "chunked", "multiple", "custom", "invalid", "all"],
                        default="all", help="选择要运行的测试")
    
    args = parser.parse_args()
    
    print(f"[*] 连接到服务器: {args.host}:{args.port}")
    
    tests = {
        "simple": lambda: test_simple_message(args.host, args.port),
        "chunked": lambda: test_chunked_message(args.host, args.port),
        "multiple": lambda: test_multiple_connections(args.host, args.port),
        "custom": lambda: test_custom_message(args.host, args.port),
        "invalid": test_invalid_magic,
    }
    
    if args.test == "all":
        for test_name, test_func in tests.items():
            if test_name != "custom":  # 跳过需要交互的测试
                try:
                    test_func()
                except Exception as e:
                    print(f"[!] 测试 {test_name} 失败: {e}")
    else:
        try:
            tests[args.test]()
        except Exception as e:
            print(f"[!] 测试失败: {e}")
    
    print("\n[*] 测试完成")


if __name__ == "__main__":
    main()
