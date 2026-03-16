/**
  ******************************************************************************
  * @file     usart.h
  * @brief    This file contains all the function prototypes for the usart.c file.
  ******************************************************************************
  */
#ifndef __USART_H__
#define __USART_H__

#ifdef __cplusplus
extern "C" {
#endif

#include "board.h"
#include "stm32h7xx_hal_uart.h"

extern UART_HandleTypeDef huart1;

void MX_USART1_UART_Init(void);

#ifdef __cplusplus
}
#endif

#endif /* __USART_H__ */