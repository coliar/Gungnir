#include "main.h"
#include "gpio.h"
#include "usart.h"
#include "fmc.h"
#include "bsp_sdram.h"
#include "printf.h"
#include <stdint.h>

__attribute__((unused)) void led_twinkle(uint32_t ms) {
  HAL_GPIO_TogglePin(LED_GPIO_Port,LED_Pin);
  HAL_Delay(ms);
}

__attribute__((unused)) void led_toggle() {
  HAL_GPIO_TogglePin(LED_GPIO_Port,LED_Pin);
}

void enable_irq() {
  __enable_irq();
}

void disable_irq() {
  __disable_irq();
}

void _putchar(char ch) {
  if (ch == '\n') {
    char t = '\r';
    HAL_UART_Transmit(&huart1 , (uint8_t *)&t, 1, 0xFFFF);
  }

  HAL_UART_Transmit(&huart1 , (uint8_t *)&ch, 1, 0xFFFF);

  if (ch == '\r') {
    char t = '\n';
    HAL_UART_Transmit(&huart1 , (uint8_t *)&t, 1, 0xFFFF);
  }
}

void enter_sleep_mode(void) {
    // __disable_irq();
    
    // 设置 Sleep Mode，等待中断触发
    SCB->SCR |= SCB_SCR_SLEEPONEXIT_Msk;  // 使能退出时进入睡眠
    __WFI();  // 等待中断，CPU 进入 Sleep 模式
}