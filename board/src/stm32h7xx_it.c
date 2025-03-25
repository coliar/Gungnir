/**
  ******************************************************************************
  * @file    stm32h7xx_it.c
  * @brief   Interrupt Service Routines.
  ******************************************************************************
  * @attention
  *
  * Copyright (c) 2024 STMicroelectronics.
  * All rights reserved.
  *
  * This software is licensed under terms that can be found in the LICENSE file
  * in the root directory of this software component.
  * If no LICENSE file comes with this software, it is provided AS-IS.
  *
  ******************************************************************************
  */

#include "main.h"
#include "stm32h7xx_it.h"
#include <stdio.h>
#include "printf.h"


/******************************************************************************/
/*           Cortex Processor Interruption and Exception Handlers          */
/******************************************************************************/
/**
  * @brief This function handles Non maskable interrupt.
  */
void NMI_Handler(void)
{
  printf_("in NMI_Handler\n");
  /* USER CODE BEGIN NonMaskableInt_IRQn 0 */

  /* USER CODE END NonMaskableInt_IRQn 0 */
  /* USER CODE BEGIN NonMaskableInt_IRQn 1 */
  while (1)
  {
  }
  /* USER CODE END NonMaskableInt_IRQn 1 */
}

typedef struct {
  uint32_t r0;
  uint32_t r1;
  uint32_t r2;
  uint32_t r3;
  uint32_t r12;
  uint32_t lr;  // Link register
  uint32_t pc;  // Program counter
  uint32_t psr; // Program status register
} HardFaultStackFrame;

void HardFault_HandlerC(HardFaultStackFrame *stackFrame) {
  volatile uint32_t r0  = stackFrame->r0;
  volatile uint32_t r1  = stackFrame->r1;
  volatile uint32_t r2  = stackFrame->r2;
  volatile uint32_t r3  = stackFrame->r3;
  volatile uint32_t r12 = stackFrame->r12;
  volatile uint32_t lr  = stackFrame->lr;
  volatile uint32_t pc  = stackFrame->pc;
  volatile uint32_t psr = stackFrame->psr;

  volatile uint32_t cfsr = SCB->CFSR;
  volatile uint32_t hfsr = SCB->HFSR;
  volatile uint32_t mmfar = SCB->MMFAR;
  volatile uint32_t bfar = SCB->BFAR;
  //volatile uint32_t ccr = SCB->CCR;

  printf_("in HardFault_Handler:\n");
  printf_("R0  : 0x%08X\n", r0);
  printf_("R1  : 0x%08X\n", r1);
  printf_("R2  : 0x%08X\n", r2);
  printf_("R3  : 0x%08X\n", r3);
  printf_("R12 : 0x%08X\n", r12);
  printf_("LR  : 0x%08X\n", lr);
  printf_("PC  : 0x%08X\n", pc);
  printf_("PSR : 0x%08X\n", psr);

  printf_("CFSR: 0x%08X\n", cfsr);
  printf_("HFSR: 0x%08X\n", hfsr);
  //printf_("CCR : 0x%08X\n", ccr);

  if (cfsr & (1 << 7)) { // MMARVALID bit
      printf_("MMFAR: 0x%08X\n", mmfar);
  }
  if (cfsr & (1 << 15)) { // BFARVALID bit
      printf_("BFAR: 0x%08X\n", bfar);
  }

  while (1) ;
}


/**
  * @brief This function handles Hard fault interrupt.
  */
void HardFault_Handler(void)
{
  __asm volatile (
    "TST lr, #4 \n"
    "ITE EQ \n"
    "MRSEQ r0, MSP \n"
    "MRSNE r0, PSP \n"
    "B HardFault_HandlerC"
  );
}

/**
  * @brief This function handles Memory management fault.
  */
void MemManage_Handler(void)
{
  printf_("in MemManage_Handler\n");
  /* USER CODE BEGIN MemoryManagement_IRQn 0 */

  /* USER CODE END MemoryManagement_IRQn 0 */
  while (1)
  {
    /* USER CODE BEGIN W1_MemoryManagement_IRQn 0 */
    /* USER CODE END W1_MemoryManagement_IRQn 0 */
  }
}

/**
  * @brief This function handles Pre-fetch fault, memory access fault.
  */
void BusFault_Handler(void)
{
  printf_("in BusFault_Handler\n");
  /* USER CODE BEGIN BusFault_IRQn 0 */

  /* USER CODE END BusFault_IRQn 0 */
  while (1)
  {
    /* USER CODE BEGIN W1_BusFault_IRQn 0 */
    /* USER CODE END W1_BusFault_IRQn 0 */
  }
}

/**
  * @brief This function handles Undefined instruction or illegal state.
  */
void UsageFault_Handler(void)
{
  printf_("in UsageFault_Handler\n");
  /* USER CODE BEGIN UsageFault_IRQn 0 */

  /* USER CODE END UsageFault_IRQn 0 */
  while (1)
  {
    /* USER CODE BEGIN W1_UsageFault_IRQn 0 */
    /* USER CODE END W1_UsageFault_IRQn 0 */
  }
}

/**
  * @brief This function handles System service call via SWI instruction.
  */
void SVC_Handler(void)
{
  printf_("in SVC_Handler\n");
  /* USER CODE BEGIN SVCall_IRQn 0 */

  /* USER CODE END SVCall_IRQn 0 */
  /* USER CODE BEGIN SVCall_IRQn 1 */

  /* USER CODE END SVCall_IRQn 1 */
}

/**
  * @brief This function handles Debug monitor.
  */
void DebugMon_Handler(void)
{
  printf_("in DebugMon_Handler\n");
  /* USER CODE BEGIN DebugMonitor_IRQn 0 */

  /* USER CODE END DebugMonitor_IRQn 0 */
  /* USER CODE BEGIN DebugMonitor_IRQn 1 */

  /* USER CODE END DebugMonitor_IRQn 1 */
}

/**
  * @brief This function handles Pendable request for system service.
  */
void PendSV_Handler(void)
{
  printf_("in PendSV_Handler\n");
  /* USER CODE BEGIN PendSV_IRQn 0 */

  /* USER CODE END PendSV_IRQn 0 */
  /* USER CODE BEGIN PendSV_IRQn 1 */

  /* USER CODE END PendSV_IRQn 1 */
}

/**
  * @brief This function handles System tick timer.
  */
void SysTick_Handler(void)
{
  /* USER CODE BEGIN SysTick_IRQn 0 */

  /* USER CODE END SysTick_IRQn 0 */
  HAL_IncTick();
  /* USER CODE BEGIN SysTick_IRQn 1 */
  extern void sys_tick_handler();
  sys_tick_handler();
  /* USER CODE END SysTick_IRQn 1 */
}

/******************************************************************************/
/* STM32H7xx Peripheral Interrupt Handlers                                    */
/* Add here the Interrupt Handlers for the used peripherals.                  */
/* For the available peripheral interrupt handler names,                      */
/* please refer to the startup file (startup_stm32h7xx.s).                    */
/******************************************************************************/

/* USER CODE BEGIN 1 */

/* USER CODE END 1 */
